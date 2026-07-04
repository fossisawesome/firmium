package com.fossisawesome.firmium.wear

import android.content.Context
import com.fossisawesome.firmium.data.storage.WatchPreferences
import com.google.android.gms.tasks.Tasks
import com.google.android.gms.wearable.DataClient
import com.google.android.gms.wearable.DataEvent
import com.google.android.gms.wearable.DataMap
import com.google.android.gms.wearable.DataMapItem
import com.google.android.gms.wearable.Wearable
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

// Watch-side receiver for the settings/theme the phone pushes over the Wearable Data Layer.
// Writes into the shared WatchPreferences; WatchPlaybackController already reactively collects
// the playback-related fields (sub-project 3), and FirmiumWearTheme reactively collects the
// theme fields, so no further wiring is needed once these are written.
class WearSettingsClient(context: Context, private val prefs: WatchPreferences) {

    private val dataClient = Wearable.getDataClient(context)
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

    private val listener = DataClient.OnDataChangedListener { events ->
        for (event in events) {
            if (event.type == DataEvent.TYPE_CHANGED &&
                event.dataItem.uri.path == WearContract.SETTINGS_PATH
            ) {
                applyDataMap(DataMapItem.fromDataItem(event.dataItem).dataMap)
            }
        }
    }

    fun start() {
        dataClient.addListener(listener)
        scope.launch { loadCurrent() }
    }

    fun stop() {
        dataClient.removeListener(listener)
    }

    private fun loadCurrent() {
        try {
            val buffer = Tasks.await(dataClient.dataItems)
            try {
                for (item in buffer) {
                    if (item.uri.path == WearContract.SETTINGS_PATH) {
                        applyDataMap(DataMapItem.fromDataItem(item).dataMap)
                    }
                }
            } finally {
                buffer.release()
            }
        } catch (_: Exception) {
        }
    }

    private fun applyDataMap(map: DataMap) {
        scope.launch {
            prefs.setCrossfadeEnabled(map.getBoolean(WearContract.KEY_CROSSFADE_ENABLED))
            prefs.setCrossfadeDuration(map.getInt(WearContract.KEY_CROSSFADE_DURATION))
            prefs.setCrossfadeCurve(map.getString(WearContract.KEY_CROSSFADE_CURVE) ?: "linear")
            prefs.setGaplessEnabled(map.getBoolean(WearContract.KEY_GAPLESS_ENABLED))
            prefs.setReplayGainEnabled(map.getBoolean(WearContract.KEY_REPLAY_GAIN_ENABLED))
            prefs.setDownloadFormat(map.getString(WearContract.KEY_DOWNLOAD_FORMAT) ?: "original")
            val bg = map.getString(WearContract.KEY_THEME_BG) ?: return@launch
            val surface = map.getString(WearContract.KEY_THEME_SURFACE) ?: return@launch
            val surface2 = map.getString(WearContract.KEY_THEME_SURFACE2) ?: return@launch
            val text = map.getString(WearContract.KEY_THEME_TEXT) ?: return@launch
            val muted = map.getString(WearContract.KEY_THEME_MUTED) ?: return@launch
            val accent = map.getString(WearContract.KEY_THEME_ACCENT) ?: return@launch
            val error = map.getString(WearContract.KEY_THEME_ERROR) ?: return@launch
            prefs.setThemeColors(
                bg = bg, surface = surface, surface2 = surface2, text = text,
                muted = muted, accent = accent, error = error,
                isDark = map.getBoolean(WearContract.KEY_THEME_IS_DARK),
            )
        }
    }
}
