package com.fossisawesome.firmium.data.storage

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.*
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

private val Context.dataStore: DataStore<Preferences> by preferencesDataStore("firmium_prefs")

// Non-sensitive app preferences stored in DataStore.
// Credentials go in SecureStorage; everything else lives here.
class AppPreferences(context: Context) {

    private val store = context.dataStore

    companion object {
        val SERVER_URL = stringPreferencesKey("server_url")
        val USERNAME = stringPreferencesKey("username")
        val VOLUME = floatPreferencesKey("volume")
        val CROSSFADE_ENABLED = booleanPreferencesKey("crossfade_enabled")
        val CROSSFADE_DURATION = intPreferencesKey("crossfade_duration_ms")
        val GAPLESS_ENABLED = booleanPreferencesKey("gapless_enabled")
        val REPEAT_MODE = stringPreferencesKey("repeat_mode")
        val SHUFFLE_ENABLED = booleanPreferencesKey("shuffle_enabled")
        val THEME_ID = stringPreferencesKey("theme_id")
        val UI_THEME_ID = stringPreferencesKey("ui_theme_id")
        val LRCLIB_ENABLED = booleanPreferencesKey("lrclib_enabled")
        val LYRICS_WORD_FILL_ENABLED = booleanPreferencesKey("lyrics_word_fill_enabled")
        val LASTFM_ENABLED = booleanPreferencesKey("lastfm_enabled")
        val AUTO_LOGIN = booleanPreferencesKey("auto_login")
        // Whether to persist the password to the keyring (default true for backwards compat).
        val SAVE_PASSWORD = booleanPreferencesKey("save_password")
        // Playlists stored as a Gson JSON array of Playlist objects.
        val PLAYLISTS_JSON = stringPreferencesKey("playlists_json")
        // Format used for track/album downloads: "original" (raw), "mp3", "flac", "wav", or "opus".
        val DOWNLOAD_FORMAT = stringPreferencesKey("download_format")
        val BIT_PERFECT_MODE = stringPreferencesKey("bit_perfect_mode")
    }

    val serverUrl: Flow<String?> = store.data.map { it[SERVER_URL] }
    val username: Flow<String?> = store.data.map { it[USERNAME] }
    val volume: Flow<Float> = store.data.map { it[VOLUME] ?: 1f }
    val crossfadeEnabled: Flow<Boolean> = store.data.map { it[CROSSFADE_ENABLED] ?: false }
    val crossfadeDuration: Flow<Int> = store.data.map { it[CROSSFADE_DURATION] ?: 3000 }
    val gaplessEnabled: Flow<Boolean> = store.data.map { it[GAPLESS_ENABLED] ?: true }
    val repeatMode: Flow<String> = store.data.map { it[REPEAT_MODE] ?: "none" }
    val shuffleEnabled: Flow<Boolean> = store.data.map { it[SHUFFLE_ENABLED] ?: false }
    val themeId: Flow<String> = store.data.map { it[THEME_ID] ?: "firmium" }
    // "firmium" = icon-only bottom nav with monospace player; "material3" = standard M3 components
    val uiThemeId: Flow<String> = store.data.map { it[UI_THEME_ID] ?: "material3" }
    val lrclibEnabled: Flow<Boolean> = store.data.map { it[LRCLIB_ENABLED] ?: true }
    val lyricsWordFillEnabled: Flow<Boolean> = store.data.map { it[LYRICS_WORD_FILL_ENABLED] ?: true }
    val lastfmEnabled: Flow<Boolean> = store.data.map { it[LASTFM_ENABLED] ?: false }
    val autoLoginEnabled: Flow<Boolean> = store.data.map { it[AUTO_LOGIN] ?: true }
    // Default true so existing users who already have a saved password stay logged in.
    val savePasswordEnabled: Flow<Boolean> = store.data.map { it[SAVE_PASSWORD] ?: true }
    val playlistsJson: Flow<String?> = store.data.map { it[PLAYLISTS_JSON] }
    val downloadFormat: Flow<String> = store.data.map { it[DOWNLOAD_FORMAT] ?: "original" }
    val bitPerfectMode: Flow<String> = store.data.map { it[BIT_PERFECT_MODE] ?: "off" }

    suspend fun setServerUrl(url: String) = store.edit { it[SERVER_URL] = url }
    suspend fun setUsername(name: String) = store.edit { it[USERNAME] = name }
    suspend fun setVolume(v: Float) = store.edit { it[VOLUME] = v }
    suspend fun setCrossfadeEnabled(v: Boolean) = store.edit { it[CROSSFADE_ENABLED] = v }
    suspend fun setCrossfadeDuration(ms: Int) = store.edit { it[CROSSFADE_DURATION] = ms }
    suspend fun setGaplessEnabled(v: Boolean) = store.edit { it[GAPLESS_ENABLED] = v }
    suspend fun setRepeatMode(mode: String) = store.edit { it[REPEAT_MODE] = mode }
    suspend fun setShuffleEnabled(v: Boolean) = store.edit { it[SHUFFLE_ENABLED] = v }
    suspend fun setThemeId(id: String) = store.edit { it[THEME_ID] = id }
    suspend fun setUiThemeId(id: String) = store.edit { it[UI_THEME_ID] = id }
    suspend fun setLrclibEnabled(v: Boolean) = store.edit { it[LRCLIB_ENABLED] = v }
    suspend fun setLyricsWordFillEnabled(v: Boolean) = store.edit { it[LYRICS_WORD_FILL_ENABLED] = v }
    suspend fun setLastfmEnabled(v: Boolean) = store.edit { it[LASTFM_ENABLED] = v }
    suspend fun setAutoLoginEnabled(v: Boolean) = store.edit { it[AUTO_LOGIN] = v }
    suspend fun setSavePasswordEnabled(v: Boolean) = store.edit { it[SAVE_PASSWORD] = v }
    suspend fun setPlaylistsJson(json: String) = store.edit { it[PLAYLISTS_JSON] = json }
    suspend fun setDownloadFormat(format: String) = store.edit { it[DOWNLOAD_FORMAT] = format }
    suspend fun setBitPerfectMode(mode: String) = store.edit { it[BIT_PERFECT_MODE] = mode }

    suspend fun clear() = store.edit { it.clear() }
}
