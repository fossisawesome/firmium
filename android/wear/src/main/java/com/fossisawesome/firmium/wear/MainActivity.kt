package com.fossisawesome.firmium.wear

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.lifecycle.lifecycleScope
import com.fossisawesome.firmium.audio.WatchNowPlayingNotifier
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    private lateinit var client: WearPlaybackClient
    private lateinit var authClient: WearAuthClient

    private val playPauseReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            (application as FirmiumWearApplication).playbackController.togglePlayPause()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val app = application as FirmiumWearApplication
        client = WearPlaybackClient(applicationContext)
        authClient = WearAuthClient(applicationContext, app.secureStorage, app.authManager)

        val filter = IntentFilter(WatchNowPlayingNotifier.actionTogglePlayPause(packageName))
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(playPauseReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            @Suppress("UnspecifiedRegisterReceiverFlag")
            registerReceiver(playPauseReceiver, filter)
        }

        setContent {
            FirmiumWearTheme {
                RemoteScreen(client)
            }
        }
    }

    // Only listen while the watch UI is visible — this is a foreground remote, not a service.
    override fun onStart() {
        super.onStart()
        client.start()
        authClient.start()
        // Temporary verification hook for the watch playback engine (sub-project 3) — plays the
        // first track of the first album from the first artist, confirming
        // WatchPlaybackController -> AudioPlayer -> ExoPlayer -> WatchNowPlayingNotifier works
        // end-to-end against a real server. Remove once sub-project 4 (browse UI) gives the
        // watch a real playback entry point.
        lifecycleScope.launch {
            val app = application as FirmiumWearApplication
            try {
                val artist = app.api.getArtists().firstOrNull() ?: return@launch
                val album = app.api.getArtistDetail(artist.id).albums.firstOrNull() ?: return@launch
                val tracks = app.api.getAlbumDetail(album.id).tracks
                if (tracks.isNotEmpty()) {
                    android.util.Log.d("FirmiumWear", "playing ${tracks.first().title} from ${album.name}")
                    app.playbackController.playAt(tracks, 0)
                }
            } catch (e: Exception) {
                android.util.Log.d("FirmiumWear", "playback verification failed", e)
            }
        }
    }

    override fun onStop() {
        client.stop()
        authClient.stop()
        super.onStop()
    }

    override fun onDestroy() {
        unregisterReceiver(playPauseReceiver)
        super.onDestroy()
    }
}
