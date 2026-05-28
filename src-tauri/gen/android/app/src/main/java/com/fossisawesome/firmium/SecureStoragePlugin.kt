package com.fossisawesome.firmium

import android.app.Activity
import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class PasswordArgs {
    var service: String = ""
    var user: String = ""
    var pass: String = ""
}

// Wraps Android EncryptedSharedPreferences (backed by Android Keystore AES256-GCM)
// to provide save/get/delete for Subsonic credentials.
@TauriPlugin
class SecureStoragePlugin(private val activity: Activity) : Plugin(activity) {

    private fun getPrefs(): android.content.SharedPreferences {
        val masterKey = MasterKey.Builder(activity)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        return EncryptedSharedPreferences.create(
            activity,
            "firmium_secure_prefs",
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
    }

    @Command
    fun savePassword(invoke: Invoke) {
        val args = invoke.parseArgs(PasswordArgs::class.java)
        val key = "${args.service}::${args.user}"
        getPrefs().edit().putString(key, args.pass).apply()
        invoke.resolve()
    }

    @Command
    fun getPassword(invoke: Invoke) {
        val args = invoke.parseArgs(PasswordArgs::class.java)
        val key = "${args.service}::${args.user}"
        val pass = getPrefs().getString(key, null)
        if (pass != null) {
            val result = JSObject()
            result.put("value", pass)
            invoke.resolve(result)
        } else {
            invoke.reject("No credential stored for $key")
        }
    }

    @Command
    fun deletePassword(invoke: Invoke) {
        val args = invoke.parseArgs(PasswordArgs::class.java)
        val key = "${args.service}::${args.user}"
        getPrefs().edit().remove(key).apply()
        invoke.resolve()
    }
}
