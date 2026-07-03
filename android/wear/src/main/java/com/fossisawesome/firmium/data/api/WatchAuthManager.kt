package com.fossisawesome.firmium.data.api

import com.fossisawesome.firmium.wear.WatchSecureStorage
import java.security.MessageDigest
import java.util.UUID

// Watch-side counterpart to the phone's AuthManager. Holds credentials and builds
// OpenSubsonic request URLs/tokens the same way, but has no login/persist/switch-account
// methods — the watch never logs in itself. Credentials arrive only via WearAuthClient
// writing into WatchSecureStorage (see the credential-handoff spec); refresh() re-reads
// after every push/clear.
class WatchAuthManager(private val secureStorage: WatchSecureStorage) {

    data class Credentials(val server: String, val username: String, val password: String)

    @Volatile
    var credentials: Credentials? = null
        private set

    // Stable auth token used exclusively for cover art URLs so image loaders can cache by
    // URL. Fresh tokens per-request would break the URL cache key.
    @Volatile
    private var stableCoverToken: Triple<String, String, String>? = null  // (salt, token, credKey)

    init {
        refresh()
    }

    // Re-reads from WatchSecureStorage. Call after WearAuthClient saves or clears credentials
    // so a running ApiClient picks up the change without an app restart.
    fun refresh() {
        val stored = secureStorage.load()
        credentials = stored?.let { Credentials(it.serverUrl.trimEnd('/'), it.username, it.password) }
        stableCoverToken = null
    }

    fun authParams(): Map<String, String> {
        val creds = credentials ?: error("Not authenticated")
        val salt = UUID.randomUUID().toString().replace("-", "").take(8)
        val token = md5("${creds.password}$salt")
        return mapOf(
            "u" to creds.username,
            "t" to token,
            "s" to salt,
            "v" to "1.16.1",
            "c" to "firmium",
            "f" to "json",
        )
    }

    fun buildUrl(action: String, extra: Map<String, String> = emptyMap()): String =
        buildUrl(action, extra.toList())

    fun buildUrl(action: String, extra: List<Pair<String, String>>): String {
        val creds = credentials ?: error("Not authenticated")
        val params = (authParams().toList() + extra)
            .joinToString("&") { (k, v) -> "$k=${v.encodeUrl()}" }
        return "${creds.server}/rest/$action?$params"
    }

    fun streamUrl(songId: String, maxBitRate: Int? = null): String {
        val extra = buildMap<String, String> {
            put("id", songId)
            if (maxBitRate != null) put("maxBitRate", maxBitRate.toString())
        }
        return buildUrl("stream", extra)
    }

    fun downloadUrl(songId: String, format: String): String {
        val fmt = if (format == "original") "raw" else format
        return buildUrl("stream", mapOf("id" to songId, "format" to fmt))
    }

    fun coverArtUrl(coverId: String, size: Int? = null): String {
        val creds = credentials ?: error("Not authenticated")
        val credKey = "${creds.server}:${creds.username}"
        val (salt, token, _) = stableCoverToken?.takeIf { it.third == credKey }
            ?: run {
                val s = UUID.randomUUID().toString().replace("-", "").take(8)
                val t = md5("${creds.password}$s")
                Triple(s, t, credKey).also { stableCoverToken = it }
            }
        val params = buildMap<String, String> {
            put("u", creds.username); put("t", token); put("s", salt)
            put("v", "1.16.1"); put("c", "firmium"); put("f", "json")
            put("id", coverId)
            if (size != null) put("size", size.toString())
        }.entries.joinToString("&") { (k, v) -> "$k=${v.encodeUrl()}" }
        return "${creds.server}/rest/getCoverArt?$params"
    }

    private fun md5(input: String): String {
        val bytes = MessageDigest.getInstance("MD5").digest(input.toByteArray())
        return bytes.joinToString("") { "%02x".format(it) }
    }

    private fun String.encodeUrl(): String =
        java.net.URLEncoder.encode(this, "UTF-8")
}
