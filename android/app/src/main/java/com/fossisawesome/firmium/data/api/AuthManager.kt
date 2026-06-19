package com.fossisawesome.firmium.data.api

import com.fossisawesome.firmium.data.storage.AppPreferences
import com.fossisawesome.firmium.data.storage.SecureStorage
import kotlinx.coroutines.flow.first
import org.json.JSONArray
import org.json.JSONObject
import java.security.MessageDigest
import java.util.UUID

// Holds current session credentials and generates Subsonic auth params.
// Mirrors generate_auth_params from lib.rs: token = MD5(password + salt).
class AuthManager(
    private val secureStorage: SecureStorage,
    private val prefs: AppPreferences,
) {

    data class Credentials(val server: String, val username: String, val password: String)

    @Volatile
    private var _credentials: Credentials? = null

    // Stable auth token used exclusively for cover art URLs so Coil can cache by URL.
    // Fresh tokens per-request would break the URL cache key.
    @Volatile
    private var _stableCoverToken: Triple<String, String, String>? = null  // (salt, token, credKey)

    val credentials: Credentials? get() = _credentials
    val isAuthenticated: Boolean get() = _credentials != null

    // Called on login or app resume after restoring saved credentials.
    fun setCredentials(server: String, username: String, password: String) {
        _credentials = Credentials(server.trimEnd('/'), username, password)
        _stableCoverToken = null  // invalidate on credential change
    }

    fun clearCredentials() {
        _credentials?.let { secureStorage.delete("firmium", it.username) }
        _credentials = null
    }

    // Attempts to restore credentials from secure storage on app start.
    // Returns true if credentials were found and loaded.
    suspend fun tryRestoreCredentials(): Boolean {
        val server = prefs.serverUrl.first() ?: return false
        val username = prefs.username.first() ?: return false
        val password = secureStorage.get("firmium", username) ?: return false
        _credentials = Credentials(server.trimEnd('/'), username, password)
        return true
    }

    // Saves credentials to persistent storage after successful login.
    // Server URL and username are always saved; password is only saved if savePassword=true.
    suspend fun persistCredentials(server: String, username: String, password: String, savePassword: Boolean = true) {
        setCredentials(server, username, password)
        val normalized = server.trimEnd('/')
        prefs.setServerUrl(normalized)
        prefs.setUsername(username)
        val key = serverStorageKey(normalized, username)
        if (savePassword) {
            secureStorage.save("firmium", username, password)
            secureStorage.save(key, username, password)
        } else {
            secureStorage.delete("firmium", username)
            secureStorage.delete(key, username)
        }
        addToServerList(normalized, username)
    }

    data class SavedServer(val url: String, val username: String)

    suspend fun savedServers(): List<SavedServer> {
        val json = prefs.serverListJson.first() ?: return emptyList()
        return try {
            val arr = JSONArray(json)
            (0 until arr.length()).map { i ->
                val obj = arr.getJSONObject(i)
                SavedServer(obj.getString("url"), obj.getString("username"))
            }
        } catch (_: Exception) { emptyList() }
    }

    private suspend fun addToServerList(url: String, username: String) {
        val list = savedServers().toMutableList()
        list.removeAll { it.url == url && it.username == username }
        list.add(0, SavedServer(url, username))
        prefs.setServerListJson(serverListToJson(list))
    }

    suspend fun removeFromServerList(url: String, username: String) {
        val key = serverStorageKey(url, username)
        secureStorage.delete(key, username)
        val list = savedServers().filter { !(it.url == url && it.username == username) }
        prefs.setServerListJson(serverListToJson(list))
    }

    suspend fun switchToSaved(url: String, username: String): Boolean {
        val key = serverStorageKey(url, username)
        val password = secureStorage.get(key, username)
            ?: secureStorage.get("firmium", username)
            ?: return false
        setCredentials(url, username, password)
        prefs.setServerUrl(url)
        prefs.setUsername(username)
        addToServerList(url, username)
        return true
    }

    private fun serverStorageKey(url: String, username: String): String =
        "firmium::$url"

    private fun serverListToJson(list: List<SavedServer>): String {
        val arr = JSONArray()
        list.forEach { s ->
            arr.put(JSONObject().put("url", s.url).put("username", s.username))
        }
        return arr.toString()
    }

    // Builds the full auth query param map for a Subsonic request.
    fun authParams(): Map<String, String> {
        val creds = _credentials ?: error("Not authenticated")
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

    // Builds a full Subsonic REST URL: ${server}/rest/${action}?${authParams}&${extraParams}
    fun buildUrl(action: String, extra: Map<String, String> = emptyMap()): String =
        buildUrl(action, extra.toList())

    // Overload accepting a list of pairs so callers can pass repeated query params
    // (e.g. multiple songIdToAdd entries for updatePlaylist), which a Map can't represent.
    fun buildUrl(action: String, extra: List<Pair<String, String>>): String {
        val creds = _credentials ?: error("Not authenticated")
        val params = (authParams().toList() + extra)
            .joinToString("&") { (k, v) -> "$k=${v.encodeUrl()}" }
        return "${creds.server}/rest/$action?$params"
    }

    // Stream URL for a track — passed to ExoPlayer directly.
    fun streamUrl(songId: String, maxBitRate: Int? = null): String {
        val extra = buildMap<String, String> {
            put("id", songId)
            if (maxBitRate != null) put("maxBitRate", maxBitRate.toString())
        }
        return buildUrl("stream", extra)
    }

    // Download URL for a track. "original" maps to format=raw (server's source file).
    fun downloadUrl(songId: String, format: String): String {
        val fmt = if (format == "original") "raw" else format
        return buildUrl("stream", mapOf("id" to songId, "format" to fmt))
    }

    // Cover art URL — stable within a session so Coil can cache by URL.
    // Uses a per-session fixed salt+token rather than a fresh UUID each call.
    fun coverArtUrl(coverId: String, size: Int? = null): String {
        val creds = _credentials ?: error("Not authenticated")
        val credKey = "${creds.server}:${creds.username}"
        val (salt, token, _) = _stableCoverToken?.takeIf { it.third == credKey }
            ?: run {
                val s = UUID.randomUUID().toString().replace("-", "").take(8)
                val t = md5("${creds.password}$s")
                Triple(s, t, credKey).also { _stableCoverToken = it }
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
