package com.fossisawesome.firmium

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.fossisawesome.firmium.audio.NowPlayingController
import com.fossisawesome.firmium.ui.theme.FirmiumTheme
import com.fossisawesome.firmium.ui.theme.LocalFirmiumColors
import com.fossisawesome.firmium.ui.tv.TvLoginScreen
import com.fossisawesome.firmium.ui.tv.TvNavGraph
import com.fossisawesome.firmium.viewmodel.AuthViewModel
import com.fossisawesome.firmium.viewmodel.LibraryViewModel
import com.fossisawesome.firmium.viewmodel.PlayerViewModel
import com.fossisawesome.firmium.viewmodel.PlaylistViewModel
import com.fossisawesome.firmium.viewmodel.SearchViewModel

// Android TV entry point — separate leanback-launcher activity from MainActivity, reusing the
// same ViewModels/data/audio layer. Phone-only concerns (runtime permission flow, onboarding,
// account-switcher dialog, edge-to-edge) are intentionally omitted; see FEATURES.md for scope.
class TvMainActivity : ComponentActivity() {

    private val app get() = application as FirmiumApplication

    // Hardware play/pause on the remote must work regardless of which screen is showing.
    private val mediaReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            val action = when (intent.action) {
                NowPlayingController.actionPrev(packageName) -> "prev"
                NowPlayingController.actionPlayPause(packageName) -> "togglePlayPause"
                NowPlayingController.actionNext(packageName) -> "next"
                NowPlayingController.actionShuffle(packageName) -> "shuffle"
                NowPlayingController.actionRepeat(packageName) -> "repeat"
                NowPlayingController.actionDismiss(packageName) -> { app.nowPlaying.clear(); return }
                else -> return
            }
            app.nowPlaying.handleAction(action)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val filter = IntentFilter().apply {
            addAction(NowPlayingController.actionPrev(packageName))
            addAction(NowPlayingController.actionPlayPause(packageName))
            addAction(NowPlayingController.actionNext(packageName))
            addAction(NowPlayingController.actionShuffle(packageName))
            addAction(NowPlayingController.actionRepeat(packageName))
            addAction(NowPlayingController.actionDismiss(packageName))
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(mediaReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            @Suppress("UnspecifiedRegisterReceiverFlag")
            registerReceiver(mediaReceiver, filter)
        }

        setContent {
            val themeId by app.prefs.themeId.collectAsStateWithLifecycle(initialValue = "firmium")
            val fontFamily by app.prefs.fontFamily.collectAsStateWithLifecycle(initialValue = "Liberation Mono")

            FirmiumTheme(themeId = themeId, fontFamily = fontFamily) {
                val colors = LocalFirmiumColors.current
                val authViewModel: AuthViewModel = viewModel()
                val authState by authViewModel.state.collectAsStateWithLifecycle()

                Box(modifier = Modifier.fillMaxSize().background(colors.bg)) {
                    if (!authState.isLoading) {
                        if (!authState.isAuthenticated || authState.needsLogin) {
                            TvLoginScreen(
                                error = authState.error,
                                onLogin = { server, username, password ->
                                    authViewModel.login(server, username, password, savePassword = true)
                                },
                            )
                        } else {
                            val playerViewModel: PlayerViewModel = viewModel()
                            val libraryViewModel: LibraryViewModel = viewModel()
                            val searchViewModel: SearchViewModel = viewModel()
                            val playlistViewModel: PlaylistViewModel = viewModel()

                            TvNavGraph(
                                auth = app.auth,
                                authViewModel = authViewModel,
                                playerViewModel = playerViewModel,
                                libraryViewModel = libraryViewModel,
                                searchViewModel = searchViewModel,
                                playlistViewModel = playlistViewModel,
                            )
                        }
                    }
                }
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        unregisterReceiver(mediaReceiver)
    }
}
