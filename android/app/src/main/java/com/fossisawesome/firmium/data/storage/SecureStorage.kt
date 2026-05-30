package com.fossisawesome.firmium.data.storage

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

// Wraps EncryptedSharedPreferences (Android Keystore AES256-GCM) for credential storage.
// Ported from SecureStoragePlugin — same key format: "${service}::${user}".
class SecureStorage(context: Context) {

    private val prefs by lazy {
        val masterKey = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        EncryptedSharedPreferences.create(
            context,
            "firmium_secure_prefs",
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
    }

    fun save(service: String, user: String, pass: String) {
        prefs.edit().putString("$service::$user", pass).apply()
    }

    fun get(service: String, user: String): String? =
        prefs.getString("$service::$user", null)

    fun delete(service: String, user: String) {
        prefs.edit().remove("$service::$user").apply()
    }
}
