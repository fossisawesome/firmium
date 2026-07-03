package com.fossisawesome.firmium.data.storage

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.*
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

private val Context.watchDataStore: DataStore<Preferences> by preferencesDataStore("firmium_watch_prefs")

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
    }

    val volume: Flow<Float> = store.data.map { it[VOLUME] ?: 1f }
    val repeatMode: Flow<String> = store.data.map { it[REPEAT_MODE] ?: "none" }
    val shuffleEnabled: Flow<Boolean> = store.data.map { it[SHUFFLE_ENABLED] ?: false }
    val crossfadeEnabled: Flow<Boolean> = store.data.map { it[CROSSFADE_ENABLED] ?: false }
    val crossfadeDuration: Flow<Int> = store.data.map { it[CROSSFADE_DURATION] ?: 3000 }
    val crossfadeCurve: Flow<String> = store.data.map { it[CROSSFADE_CURVE] ?: "linear" }
    val gaplessEnabled: Flow<Boolean> = store.data.map { it[GAPLESS_ENABLED] ?: true }
    val replayGainEnabled: Flow<Boolean> = store.data.map { it[REPLAY_GAIN_ENABLED] ?: true }

    suspend fun setVolume(v: Float) = store.edit { it[VOLUME] = v }
    suspend fun setRepeatMode(mode: String) = store.edit { it[REPEAT_MODE] = mode }
    suspend fun setShuffleEnabled(v: Boolean) = store.edit { it[SHUFFLE_ENABLED] = v }
    suspend fun setCrossfadeEnabled(v: Boolean) = store.edit { it[CROSSFADE_ENABLED] = v }
    suspend fun setCrossfadeDuration(ms: Int) = store.edit { it[CROSSFADE_DURATION] = ms }
    suspend fun setCrossfadeCurve(curve: String) = store.edit { it[CROSSFADE_CURVE] = curve }
    suspend fun setGaplessEnabled(v: Boolean) = store.edit { it[GAPLESS_ENABLED] = v }
    suspend fun setReplayGainEnabled(v: Boolean) = store.edit { it[REPLAY_GAIN_ENABLED] = v }
}
