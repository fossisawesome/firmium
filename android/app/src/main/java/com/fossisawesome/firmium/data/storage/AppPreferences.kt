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
        // Crossfade ramp shape: "linear" or "logarithmic".
        val CROSSFADE_CURVE = stringPreferencesKey("crossfade_curve")
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
        val REPLAY_GAIN_ENABLED = booleanPreferencesKey("replay_gain_enabled")
        // Smart Radio: seed and append more tracks when the queue ends (off by default).
        val AUTO_CONTINUE_ENABLED = booleanPreferencesKey("auto_continue_enabled")
        // ListenBrainz scrobbling enabled (token itself lives in SecureStorage).
        val LISTENBRAINZ_ENABLED = booleanPreferencesKey("listenbrainz_enabled")
        // First-run onboarding tour completion flag.
        val ONBOARDED = booleanPreferencesKey("onboarded")
        // Audio visualizer: off by default; type is "orb" | "bars" | "oscilloscope".
        val VISUALIZER_ENABLED = booleanPreferencesKey("visualizer_enabled")
        val VISUALIZER_TYPE = stringPreferencesKey("visualizer_type")
        // Equalizer: off by default. Profiles stored as a Gson JSON array; mode is
        // "graphic" | "parametric"; active profile referenced by name.
        val EQ_ENABLED = booleanPreferencesKey("eq_enabled")
        val EQ_MODE = stringPreferencesKey("eq_mode")
        val EQ_ACTIVE_PROFILE = stringPreferencesKey("eq_active_profile")
        val EQ_PROFILES_JSON = stringPreferencesKey("eq_profiles_json")
        val SERVER_LIST_JSON = stringPreferencesKey("server_list_json")
        // Firmium Recap weekly auto-show: unix millis of the last time it was surfaced.
        val RECAP_LAST_SHOWN = longPreferencesKey("recap_last_shown")
    }

    val serverUrl: Flow<String?> = store.data.map { it[SERVER_URL] }
    val username: Flow<String?> = store.data.map { it[USERNAME] }
    val volume: Flow<Float> = store.data.map { it[VOLUME] ?: 1f }
    val crossfadeEnabled: Flow<Boolean> = store.data.map { it[CROSSFADE_ENABLED] ?: false }
    val crossfadeDuration: Flow<Int> = store.data.map { it[CROSSFADE_DURATION] ?: 3000 }
    val crossfadeCurve: Flow<String> = store.data.map { it[CROSSFADE_CURVE] ?: "linear" }
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
    val replayGainEnabled: Flow<Boolean> = store.data.map { it[REPLAY_GAIN_ENABLED] ?: true }
    val autoContinueEnabled: Flow<Boolean> = store.data.map { it[AUTO_CONTINUE_ENABLED] ?: false }
    val listenbrainzEnabled: Flow<Boolean> = store.data.map { it[LISTENBRAINZ_ENABLED] ?: false }
    val onboarded: Flow<Boolean> = store.data.map { it[ONBOARDED] ?: false }
    val visualizerEnabled: Flow<Boolean> = store.data.map { it[VISUALIZER_ENABLED] ?: false }
    val visualizerType: Flow<String> = store.data.map { it[VISUALIZER_TYPE] ?: "orb" }
    val eqEnabled: Flow<Boolean> = store.data.map { it[EQ_ENABLED] ?: false }
    val eqMode: Flow<String> = store.data.map { it[EQ_MODE] ?: "graphic" }
    val eqActiveProfile: Flow<String?> = store.data.map { it[EQ_ACTIVE_PROFILE] }
    val eqProfilesJson: Flow<String?> = store.data.map { it[EQ_PROFILES_JSON] }
    val serverListJson: Flow<String?> = store.data.map { it[SERVER_LIST_JSON] }
    val recapLastShown: Flow<Long> = store.data.map { it[RECAP_LAST_SHOWN] ?: 0L }

    suspend fun setServerUrl(url: String) = store.edit { it[SERVER_URL] = url }
    suspend fun setUsername(name: String) = store.edit { it[USERNAME] = name }
    suspend fun setVolume(v: Float) = store.edit { it[VOLUME] = v }
    suspend fun setCrossfadeEnabled(v: Boolean) = store.edit { it[CROSSFADE_ENABLED] = v }
    suspend fun setCrossfadeDuration(ms: Int) = store.edit { it[CROSSFADE_DURATION] = ms }
    suspend fun setCrossfadeCurve(curve: String) = store.edit { it[CROSSFADE_CURVE] = curve }
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
    suspend fun setReplayGainEnabled(v: Boolean) = store.edit { it[REPLAY_GAIN_ENABLED] = v }
    suspend fun setAutoContinueEnabled(v: Boolean) = store.edit { it[AUTO_CONTINUE_ENABLED] = v }
    suspend fun setListenbrainzEnabled(v: Boolean) = store.edit { it[LISTENBRAINZ_ENABLED] = v }
    suspend fun setOnboarded(v: Boolean) = store.edit { it[ONBOARDED] = v }
    suspend fun setVisualizerEnabled(v: Boolean) = store.edit { it[VISUALIZER_ENABLED] = v }
    suspend fun setVisualizerType(t: String) = store.edit { it[VISUALIZER_TYPE] = t }
    suspend fun setEqEnabled(v: Boolean) = store.edit { it[EQ_ENABLED] = v }
    suspend fun setEqMode(mode: String) = store.edit { it[EQ_MODE] = mode }
    suspend fun setEqActiveProfile(name: String) = store.edit { it[EQ_ACTIVE_PROFILE] = name }
    suspend fun setEqProfilesJson(json: String) = store.edit { it[EQ_PROFILES_JSON] = json }
    suspend fun setServerListJson(json: String) = store.edit { it[SERVER_LIST_JSON] = json }
    suspend fun setRecapLastShown(millis: Long) = store.edit { it[RECAP_LAST_SHOWN] = millis }

    suspend fun clear() = store.edit { it.clear() }
}
