package com.fossisawesome.firmium.data.storage

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.*
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

private val Context.watchDataStore: DataStore<Preferences> by preferencesDataStore("firmium_watch_prefs")

// Resolved theme colors synced from the phone (hex strings, e.g. "#1a1a1a"). Null until the
// phone has pushed at least once — callers fall back to a hardcoded default in that case.
data class WatchThemeColors(
    val bg: String,
    val surface: String,
    val surface2: String,
    val text: String,
    val muted: String,
    val accent: String,
    val error: String,
    val isDark: Boolean,
)

// Watch-local playback settings, mirrors the phone's AppPreferences defaults but scoped to
// only what WatchPlaybackController needs — no auth/theme/other unrelated prefs.
class WatchPreferences(context: Context) {

    private val store = context.watchDataStore

    companion object {
        val VOLUME = floatPreferencesKey("volume")
        val REPEAT_MODE = stringPreferencesKey("repeat_mode")
        val SHUFFLE_ENABLED = booleanPreferencesKey("shuffle_enabled")
        val CROSSFADE_ENABLED = booleanPreferencesKey("crossfade_enabled")
        val CROSSFADE_DURATION = intPreferencesKey("crossfade_duration_ms")
        val CROSSFADE_CURVE = stringPreferencesKey("crossfade_curve")
        val GAPLESS_ENABLED = booleanPreferencesKey("gapless_enabled")
        val REPLAY_GAIN_ENABLED = booleanPreferencesKey("replay_gain_enabled")
        val LAST_MODE = stringPreferencesKey("last_mode")
        val DOWNLOAD_FORMAT = stringPreferencesKey("download_format")
        val THEME_BG = stringPreferencesKey("theme_bg")
        val THEME_SURFACE = stringPreferencesKey("theme_surface")
        val THEME_SURFACE2 = stringPreferencesKey("theme_surface2")
        val THEME_TEXT = stringPreferencesKey("theme_text")
        val THEME_MUTED = stringPreferencesKey("theme_muted")
        val THEME_ACCENT = stringPreferencesKey("theme_accent")
        val THEME_ERROR = stringPreferencesKey("theme_error")
        val THEME_IS_DARK = booleanPreferencesKey("theme_is_dark")
    }

    val volume: Flow<Float> = store.data.map { it[VOLUME] ?: 1f }
    val repeatMode: Flow<String> = store.data.map { it[REPEAT_MODE] ?: "none" }
    val shuffleEnabled: Flow<Boolean> = store.data.map { it[SHUFFLE_ENABLED] ?: false }
    val crossfadeEnabled: Flow<Boolean> = store.data.map { it[CROSSFADE_ENABLED] ?: false }
    val crossfadeDuration: Flow<Int> = store.data.map { it[CROSSFADE_DURATION] ?: 3000 }
    val crossfadeCurve: Flow<String> = store.data.map { it[CROSSFADE_CURVE] ?: "linear" }
    val gaplessEnabled: Flow<Boolean> = store.data.map { it[GAPLESS_ENABLED] ?: true }
    val replayGainEnabled: Flow<Boolean> = store.data.map { it[REPLAY_GAIN_ENABLED] ?: true }
    val lastMode: Flow<String> = store.data.map { it[LAST_MODE] ?: "standalone" }
    val downloadFormat: Flow<String> = store.data.map { it[DOWNLOAD_FORMAT] ?: "original" }
    val themeColors: Flow<WatchThemeColors?> = store.data.map { prefs ->
        val bg = prefs[THEME_BG] ?: return@map null
        WatchThemeColors(
            bg = bg,
            surface = prefs[THEME_SURFACE] ?: return@map null,
            surface2 = prefs[THEME_SURFACE2] ?: return@map null,
            text = prefs[THEME_TEXT] ?: return@map null,
            muted = prefs[THEME_MUTED] ?: return@map null,
            accent = prefs[THEME_ACCENT] ?: return@map null,
            error = prefs[THEME_ERROR] ?: return@map null,
            isDark = prefs[THEME_IS_DARK] ?: true,
        )
    }

    suspend fun setVolume(v: Float) = store.edit { it[VOLUME] = v }
    suspend fun setRepeatMode(mode: String) = store.edit { it[REPEAT_MODE] = mode }
    suspend fun setShuffleEnabled(v: Boolean) = store.edit { it[SHUFFLE_ENABLED] = v }
    suspend fun setCrossfadeEnabled(v: Boolean) = store.edit { it[CROSSFADE_ENABLED] = v }
    suspend fun setCrossfadeDuration(ms: Int) = store.edit { it[CROSSFADE_DURATION] = ms }
    suspend fun setCrossfadeCurve(curve: String) = store.edit { it[CROSSFADE_CURVE] = curve }
    suspend fun setGaplessEnabled(v: Boolean) = store.edit { it[GAPLESS_ENABLED] = v }
    suspend fun setReplayGainEnabled(v: Boolean) = store.edit { it[REPLAY_GAIN_ENABLED] = v }
    suspend fun setLastMode(mode: String) = store.edit { it[LAST_MODE] = mode }
    suspend fun setDownloadFormat(format: String) = store.edit { it[DOWNLOAD_FORMAT] = format }

    suspend fun setThemeColors(
        bg: String, surface: String, surface2: String, text: String,
        muted: String, accent: String, error: String, isDark: Boolean,
    ) {
        store.edit {
            it[THEME_BG] = bg
            it[THEME_SURFACE] = surface
            it[THEME_SURFACE2] = surface2
            it[THEME_TEXT] = text
            it[THEME_MUTED] = muted
            it[THEME_ACCENT] = accent
            it[THEME_ERROR] = error
            it[THEME_IS_DARK] = isDark
        }
    }
}
