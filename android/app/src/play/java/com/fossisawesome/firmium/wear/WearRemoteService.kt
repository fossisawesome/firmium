package com.fossisawesome.firmium.wear

import android.os.Handler
import android.os.Looper
import com.fossisawesome.firmium.FirmiumApplication
import com.google.android.gms.wearable.MessageEvent
import com.google.android.gms.wearable.WearableListenerService

// Receives transport commands from the paired watch and applies them to the shared
// PlaybackController. Declared as a WearableListenerService so the phone wakes to handle a
// command even when the app is backgrounded.
class WearRemoteService : WearableListenerService() {

    // ExoPlayer (via PlaybackController) must be touched on the main thread; onMessageReceived
    // runs on a binder thread, so hop to the main looper like the media-session callbacks do.
    private val mainHandler = Handler(Looper.getMainLooper())

    override fun onMessageReceived(event: MessageEvent) {
        if (event.path != WearContract.CMD_PATH) return
        val payload = String(event.data, Charsets.UTF_8)
        val playback = (application as FirmiumApplication).playback

        mainHandler.post {
            when {
                payload == WearContract.CMD_PLAY_PAUSE -> playback.togglePlayPause()
                payload == WearContract.CMD_NEXT -> playback.skipToNext()
                payload == WearContract.CMD_PREV -> playback.skipToPrevious()
                payload.startsWith(WearContract.CMD_SET_VOLUME_PREFIX) -> {
                    payload.removePrefix(WearContract.CMD_SET_VOLUME_PREFIX).toFloatOrNull()
                        ?.let { playback.setVolume(it.coerceIn(0f, 1f)) }
                }
            }
        }
    }
}
