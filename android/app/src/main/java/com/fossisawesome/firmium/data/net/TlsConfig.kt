package com.fossisawesome.firmium.data.net

import android.content.Context
import android.security.keystore.KeyProperties
import android.security.keystore.KeyProtection
import android.util.Log
import com.fossisawesome.firmium.BuildConfig
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import java.io.ByteArrayInputStream
import java.io.File
import java.net.Socket
import java.security.KeyStore
import java.security.Principal
import java.security.PrivateKey
import java.security.cert.CertificateException
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLContext
import javax.net.ssl.TrustManagerFactory
import javax.net.ssl.X509KeyManager
import javax.net.ssl.X509TrustManager

// Shared OkHttpClient with optional mTLS, mirroring backend/tls.rs on desktop. The client
// identity (from a PKCS#12 file) is imported into the Android Keystore under a private alias, so
// the key becomes non-extractable and TLS signing happens inside the Keystore. An optional extra
// CA (PEM) is stored in filesDir and trusted alongside the system roots. The client is rebuilt
// on every change; all HTTP stacks (API, downloads, ExoPlayer, Coil) read `client` per request.
object TlsConfig {
    private const val ALIAS = "firmium_mtls"
    private const val KEYSTORE = "AndroidKeyStore"

    private lateinit var caFile: File

    @Volatile
    private var built: OkHttpClient? = null

    /** Current shared client. Do not cache the result across calls. */
    val client: OkHttpClient
        get() = built ?: synchronized(this) { built ?: build().also { built = it } }

    fun init(context: Context) {
        caFile = File(context.filesDir, "certs/ca.pem")
    }

    private fun keyStore(): KeyStore = KeyStore.getInstance(KEYSTORE).apply { load(null) }

    fun hasIdentity(): Boolean = runCatching { keyStore().containsAlias(ALIAS) }.getOrDefault(false)
    fun hasCa(): Boolean = caFile.exists()

    /** Imports a PKCS#12 client identity. Throws with a user-readable message on failure. */
    fun importIdentity(p12: ByteArray, password: CharArray) {
        val src = KeyStore.getInstance("PKCS12")
        try {
            src.load(ByteArrayInputStream(p12), password)
        } catch (e: Exception) {
            throw IllegalArgumentException("Wrong password, or not a PKCS#12 (.p12/.pfx) file")
        }
        val srcAlias = src.aliases().toList().firstOrNull { src.isKeyEntry(it) }
            ?: throw IllegalArgumentException("No private key found in the file")
        val key = src.getKey(srcAlias, password) as PrivateKey
        val chain = src.getCertificateChain(srcAlias)

        // TLS 1.2 signs with PKCS1 (digest NONE or hashed) and TLS 1.3 with PSS; allow all of them.
        val rsa = key.algorithm == "RSA"
        val protection = KeyProtection.Builder(
            if (rsa) KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_DECRYPT else KeyProperties.PURPOSE_SIGN
        ).setDigests(
            KeyProperties.DIGEST_NONE, KeyProperties.DIGEST_SHA1, KeyProperties.DIGEST_SHA256,
            KeyProperties.DIGEST_SHA384, KeyProperties.DIGEST_SHA512,
        ).apply {
            if (rsa) {
                setSignaturePaddings(KeyProperties.SIGNATURE_PADDING_RSA_PKCS1, KeyProperties.SIGNATURE_PADDING_RSA_PSS)
                setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_RSA_PKCS1)
            }
        }.build()

        val ks = keyStore()
        ks.setEntry(ALIAS, KeyStore.PrivateKeyEntry(key, chain), protection)
        rebuild()
    }

    fun clearIdentity() {
        runCatching { keyStore().deleteEntry(ALIAS) }
        rebuild()
    }

    /** Installs an extra trusted CA (PEM bundle). Rolls back if it holds no certificates. */
    fun importCa(pem: ByteArray) {
        val certs = runCatching { CertificateFactory.getInstance("X.509").generateCertificates(ByteArrayInputStream(pem)) }
            .getOrNull().orEmpty()
        if (certs.isEmpty()) throw IllegalArgumentException("No PEM certificates found in the file")
        caFile.parentFile?.mkdirs()
        caFile.writeBytes(pem)
        rebuild()
    }

    fun clearCa() {
        caFile.delete()
        rebuild()
    }

    private fun rebuild() {
        built = null
    }

    private fun build(): OkHttpClient {
        val builder = OkHttpClient.Builder()
            .connectTimeout(15, TimeUnit.SECONDS)
            .readTimeout(30, TimeUnit.SECONDS)
        val identity = hasIdentity()
        val ca = hasCa()
        if (identity || ca) {
            try {
                val system = systemTrustManager()
                val trust = if (ca) CompositeTrustManager(system, caTrustManager()) else system
                val ctx = SSLContext.getInstance("TLS")
                ctx.init(if (identity) arrayOf(IdentityKeyManager) else null, arrayOf(trust), null)
                builder.sslSocketFactory(ctx.socketFactory, trust)
            } catch (e: Exception) {
                // A broken stored cert must not take the whole app offline; fall back to plain TLS.
                Log.w("Firmium/Tls", "mTLS config ignored", e)
            }
        }
        // BASIC logging includes the request URL, which carries the OpenSubsonic auth token (t=)
        // and salt (s=) as query params — keep this out of release logcat.
        if (BuildConfig.DEBUG) {
            val redactingLogger = HttpLoggingInterceptor.Logger { message ->
                val safe = message.replace(Regex("([?&](?:t|s)=)[^&\\s]+"), "$1<redacted>")
                Log.d("Firmium/OkHttp", safe)
            }
            builder.addInterceptor(HttpLoggingInterceptor(redactingLogger).apply { level = HttpLoggingInterceptor.Level.BASIC })
        }
        return builder.build()
    }

    private fun systemTrustManager(): X509TrustManager =
        TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm())
            .apply { init(null as KeyStore?) }
            .trustManagers.filterIsInstance<X509TrustManager>().first()

    private fun caTrustManager(): X509TrustManager {
        val store = KeyStore.getInstance(KeyStore.getDefaultType()).apply { load(null) }
        CertificateFactory.getInstance("X.509").generateCertificates(ByteArrayInputStream(caFile.readBytes()))
            .forEachIndexed { i, c -> store.setCertificateEntry("ca$i", c) }
        return TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm())
            .apply { init(store) }
            .trustManagers.filterIsInstance<X509TrustManager>().first()
    }

    // Accepts a server chain trusted by either the system roots or the user's CA.
    private class CompositeTrustManager(private val system: X509TrustManager, private val custom: X509TrustManager) : X509TrustManager {
        override fun checkClientTrusted(chain: Array<X509Certificate>, authType: String) = system.checkClientTrusted(chain, authType)
        override fun checkServerTrusted(chain: Array<X509Certificate>, authType: String) {
            try { system.checkServerTrusted(chain, authType) } catch (_: CertificateException) { custom.checkServerTrusted(chain, authType) }
        }
        override fun getAcceptedIssuers(): Array<X509Certificate> = system.acceptedIssuers + custom.acceptedIssuers
    }

    // Presents the Keystore identity for any client-cert request. The key handle never leaves the Keystore.
    private object IdentityKeyManager : X509KeyManager {
        override fun chooseClientAlias(keyType: Array<out String>?, issuers: Array<out Principal>?, socket: Socket?): String = ALIAS
        override fun getClientAliases(keyType: String?, issuers: Array<out Principal>?): Array<String> = arrayOf(ALIAS)
        override fun chooseServerAlias(keyType: String?, issuers: Array<out Principal>?, socket: Socket?): String? = null
        override fun getServerAliases(keyType: String?, issuers: Array<out Principal>?): Array<String>? = null
        override fun getCertificateChain(alias: String?): Array<X509Certificate>? =
            keyStore().getCertificateChain(ALIAS)?.filterIsInstance<X509Certificate>()?.toTypedArray()
        override fun getPrivateKey(alias: String?): PrivateKey? = keyStore().getKey(ALIAS, null) as? PrivateKey
    }
}
