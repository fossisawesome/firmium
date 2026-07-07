package com.fossisawesome.firmium.wear

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.fossisawesome.firmium.audio.WatchNowPlayingNotifier
import com.fossisawesome.firmium.wear.ui.WearNavGraph

class MainActivity : ComponentActivity() {

    private lateinit var client: WearPlaybackClient
    private lateinit var authClient: WearAuthClient
    private lateinit var settingsClient: WearSettingsClient

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
        settingsClient = WearSettingsClient(applicationContext, app.watchPreferences)

        val filter = IntentFilter(WatchNowPlayingNotifier.actionTogglePlayPause(packageName))
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(playPauseReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            @Suppress("UnspecifiedRegisterReceiverFlag")
            registerReceiver(playPauseReceiver, filter)
        }

        setContent {
            FirmiumWearTheme {
                WearNavGraph(client)
            }
        }
    }

    // Only listen while the watch UI is visible — this is a foreground remote, not a service.
    override fun onStart() {
        super.onStart()
        client.start()
        authClient.start()
        settingsClient.start()
    }

    override fun onStop() {
        client.stop()
        authClient.stop()
        settingsClient.stop()
        super.onStop()
    }

    override fun onDestroy() {
        unregisterReceiver(playPauseReceiver)
        super.onDestroy()
    }
}
