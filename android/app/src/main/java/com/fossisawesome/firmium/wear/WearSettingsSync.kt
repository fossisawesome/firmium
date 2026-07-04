package com.fossisawesome.firmium.wear

import android.content.Context
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import com.fossisawesome.firmium.data.storage.AppPreferences
import com.fossisawesome.firmium.ui.theme.themeById
import com.google.android.gms.tasks.Tasks
import com.google.android.gms.wearable.PutDataMapRequest
import com.google.android.gms.wearable.Wearable
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

// Pushes the phone's playback/appearance settings (crossfade, gapless, ReplayGain, download
// format, resolved theme colors) to a paired watch. Doesn't sync shuffle/repeat/volume — those
// are live per-device playback state with their own watch-side controls. Theme is sent as
// resolved hex colors, not a theme id, so the watch never needs its own theme catalog (handles
// user-imported custom themes too).
class WearSettingsSync(context: Context, private val prefs: AppPreferences) {

    private val dataClient = Wearable.getDataClient(context)
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

    private data class Settings(
        val themeId: String = "firmium",
        val crossfadeEnabled: Boolean = false,
        val crossfadeDuration: Int = 3000,
        val crossfadeCurve: String = "linear",
        val gaplessEnabled: Boolean = true,
        val replayGainEnabled: Boolean = true,
        val downloadFormat: String = "original",
    )

    private var current = Settings()

    // Each preference collected independently (rather than combine()'d, since there are 7 of
    // them and kotlinx.coroutines' typed combine() overloads only go up to 5) — every emission
    // updates the shared `current` snapshot and re-pushes it. DataStore flows emit their current
    // value immediately on collect, so this also serves as the "push once on app start" case.
    fun start() {
        scope.launch { prefs.themeId.collect { current = current.copy(themeId = it); push() } }
        scope.launch { prefs.crossfadeEnabled.collect { current = current.copy(crossfadeEnabled = it); push() } }
        scope.launch { prefs.crossfadeDuration.collect { current = current.copy(crossfadeDuration = it); push() } }
        scope.launch { prefs.crossfadeCurve.collect { current = current.copy(crossfadeCurve = it); push() } }
        scope.launch { prefs.gaplessEnabled.collect { current = current.copy(gaplessEnabled = it); push() } }
        scope.launch { prefs.replayGainEnabled.collect { current = current.copy(replayGainEnabled = it); push() } }
        scope.launch { prefs.downloadFormat.collect { current = current.copy(downloadFormat = it); push() } }
    }

    private fun push() {
        val theme = themeById(current.themeId)
        val request = PutDataMapRequest.create(WearContract.SETTINGS_PATH).apply {
            dataMap.putBoolean(WearContract.KEY_CROSSFADE_ENABLED, current.crossfadeEnabled)
            dataMap.putInt(WearContract.KEY_CROSSFADE_DURATION, current.crossfadeDuration)
            dataMap.putString(WearContract.KEY_CROSSFADE_CURVE, current.crossfadeCurve)
            dataMap.putBoolean(WearContract.KEY_GAPLESS_ENABLED, current.gaplessEnabled)
            dataMap.putBoolean(WearContract.KEY_REPLAY_GAIN_ENABLED, current.replayGainEnabled)
            dataMap.putString(WearContract.KEY_DOWNLOAD_FORMAT, current.downloadFormat)
            dataMap.putString(WearContract.KEY_THEME_BG, theme.bg.toHexString())
            dataMap.putString(WearContract.KEY_THEME_SURFACE, theme.surface.toHexString())
            dataMap.putString(WearContract.KEY_THEME_SURFACE2, theme.surface2.toHexString())
            dataMap.putString(WearContract.KEY_THEME_TEXT, theme.text.toHexString())
            dataMap.putString(WearContract.KEY_THEME_MUTED, theme.muted.toHexString())
            dataMap.putString(WearContract.KEY_THEME_ACCENT, theme.accent.toHexString())
            dataMap.putString(WearContract.KEY_THEME_ERROR, theme.error.toHexString())
            dataMap.putBoolean(WearContract.KEY_THEME_IS_DARK, theme.isDark)
        }.asPutDataRequest().setUrgent()
        try {
            Tasks.await(dataClient.putDataItem(request))
        } catch (e: Exception) {
            android.util.Log.d("WearSettingsSync", "putDataItem failed, ignoring", e)
        }
    }

    private fun Color.toHexString(): String =
        String.format("#%06X", 0xFFFFFF and toArgb())
}
