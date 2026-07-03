package com.fossisawesome.firmium.wear

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent

class MainActivity : ComponentActivity() {

    private lateinit var client: WearPlaybackClient
    private lateinit var authClient: WearAuthClient

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        client = WearPlaybackClient(applicationContext)
        authClient = WearAuthClient(applicationContext)
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
    }

    override fun onStop() {
        client.stop()
        authClient.stop()
        super.onStop()
    }
}
