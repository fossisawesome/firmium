package com.fossisawesome.firmium.wear

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    private lateinit var client: WearPlaybackClient
    private lateinit var authClient: WearAuthClient

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val app = application as FirmiumWearApplication
        client = WearPlaybackClient(applicationContext)
        authClient = WearAuthClient(applicationContext, app.secureStorage, app.authManager)
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
        // Temporary verification hook for the watch API client (sub-project 2) — confirms
        // WatchSecureStorage -> WatchAuthManager -> ApiClient works end-to-end against a real
        // server. Remove once sub-project 4 (browse UI) gives ApiClient a real consumer.
        lifecycleScope.launch {
            val app = application as FirmiumWearApplication
            try {
                val artists = app.api.getArtists()
                android.util.Log.d("FirmiumWear", "getArtists() returned ${artists.size} artists")
            } catch (e: Exception) {
                android.util.Log.d("FirmiumWear", "getArtists() failed", e)
            }
        }
    }

    override fun onStop() {
        client.stop()
        authClient.stop()
        super.onStop()
    }
}
