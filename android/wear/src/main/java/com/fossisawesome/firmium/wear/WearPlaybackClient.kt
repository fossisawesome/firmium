package com.fossisawesome.firmium.wear

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import com.google.android.gms.tasks.Tasks
import com.google.android.gms.wearable.Asset
import com.google.android.gms.wearable.DataClient
import com.google.android.gms.wearable.DataEvent
import com.google.android.gms.wearable.DataMap
import com.google.android.gms.wearable.DataMapItem
import com.google.android.gms.wearable.Wearable
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

// Current now-playing snapshot the watch UI renders. Mirrors the DataMap the phone pushes.
data class WatchState(
    val hasTrack: Boolean = false,
    val title: String = "",
    val artist: String = "",
    val album: String = "",
    val isPlaying: Boolean = false,
    val volume: Float = 1f,
    val art: Bitmap? = null,
)

// Watch-side bridge to the phone. Listens to the DataClient for the latest now-playing snapshot
// and sends transport commands back over the MessageClient. No playback happens on the watch.
class WearPlaybackClient(context: Context) {

    private val dataClient = Wearable.getDataClient(context)
    private val messageClient = Wearable.getMessageClient(context)
    private val nodeClient = Wearable.getNodeClient(context)
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

    private val _state = MutableStateFlow(WatchState())
    val state: StateFlow<WatchState> = _state.asStateFlow()

    // Album art only changes on track change; keep it across play/pause/volume updates.
    private var lastArtTrackId: String? = null

    private val listener = DataClient.OnDataChangedListener { events ->
        for (event in events) {
            if (event.type == DataEvent.TYPE_CHANGED &&
                event.dataItem.uri.path == WearContract.NOW_PLAYING_PATH
            ) {
                applyDataMap(DataMapItem.fromDataItem(event.dataItem).dataMap)
            }
        }
    }

    fun start() {
        dataClient.addListener(listener)
        // DataClient retains the last snapshot, so fetch it immediately on open.
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
                    if (item.uri.path == WearContract.NOW_PLAYING_PATH) {
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
        val trackId = map.getString(WearContract.KEY_TRACK_ID) ?: ""
        val keepArt = trackId == lastArtTrackId
        _state.value = WatchState(
            hasTrack = map.getBoolean(WearContract.KEY_HAS_TRACK),
            title = map.getString(WearContract.KEY_TITLE) ?: "",
            artist = map.getString(WearContract.KEY_ARTIST) ?: "",
            album = map.getString(WearContract.KEY_ALBUM) ?: "",
            isPlaying = map.getBoolean(WearContract.KEY_IS_PLAYING),
            volume = map.getFloat(WearContract.KEY_VOLUME, 1f),
            art = if (keepArt) _state.value.art else null,
        )
        val asset = map.getAsset(WearContract.KEY_ART)
        when {
            asset != null && !keepArt -> scope.launch { loadAsset(asset, trackId) }
            asset == null -> lastArtTrackId = trackId
        }
    }

    private fun loadAsset(asset: Asset, trackId: String) {
        try {
            val fd = Tasks.await(dataClient.getFdForAsset(asset)) ?: return
            val bmp = fd.inputStream.use { BitmapFactory.decodeStream(it) } ?: return
            lastArtTrackId = trackId
            _state.update { it.copy(art = bmp) }
        } catch (_: Exception) {
        }
    }

    fun sendCommand(cmd: String) {
        scope.launch {
            try {
                val nodes = Tasks.await(nodeClient.connectedNodes)
                for (node in nodes) {
                    messageClient.sendMessage(node.id, WearContract.CMD_PATH, cmd.toByteArray())
                }
            } catch (_: Exception) {
            }
        }
    }

    fun setVolume(volume: Float) = sendCommand(WearContract.CMD_SET_VOLUME_PREFIX + volume)
}
