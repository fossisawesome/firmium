package com.fossisawesome.firmium.wear

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent

class MainActivity : ComponentActivity() {

    private lateinit var client: WearPlaybackClient

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        client = WearPlaybackClient(applicationContext)
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
    }

    override fun onStop() {
        client.stop()
        super.onStop()
    }
}
