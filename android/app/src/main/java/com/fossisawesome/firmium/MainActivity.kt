package com.fossisawesome.firmium

import android.Manifest
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import android.app.AlertDialog
import androidx.compose.runtime.*
import androidx.compose.runtime.rememberCoroutineScope
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.fossisawesome.firmium.audio.NowPlayingController
import com.fossisawesome.firmium.ui.navigation.AppNavGraph
import com.fossisawesome.firmium.ui.screens.AccountDialog
import com.fossisawesome.firmium.ui.theme.FirmiumTheme
import com.fossisawesome.firmium.viewmodel.*
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    private val app get() = application as FirmiumApplication

    private val mediaReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            val action = when (intent.action) {
                NowPlayingController.actionPrev(packageName) -> "prev"
                NowPlayingController.actionPlayPause(packageName) -> "togglePlayPause"
                NowPlayingController.actionNext(packageName) -> "next"
                NowPlayingController.actionDismiss(packageName) -> { app.nowPlaying.clear(); return }
                else -> return
            }
            app.nowPlaying.handleAction(action)
        }
    }

    private val notificationPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (!granted && !shouldShowRequestPermissionRationale(Manifest.permission.POST_NOTIFICATIONS)) {
            AlertDialog.Builder(this)
                .setTitle("Notifications disabled")
                .setMessage("Enable notifications to see media controls on the lock screen.")
                .setPositiveButton("Open Settings") { _, _ ->
                    startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                        data = Uri.fromParts("package", packageName, null)
                    })
                }
                .setNegativeButton("Not now", null)
                .show()
        }
    }

    private var notificationPermissionRequested = false

    private val storagePermission: String =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) Manifest.permission.READ_MEDIA_AUDIO
        else Manifest.permission.READ_EXTERNAL_STORAGE

    private val storagePermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) app.localLibrary.invalidate()
        else if (!shouldShowRequestPermissionRationale(storagePermission)) {
            AlertDialog.Builder(this)
                .setTitle("Storage access disabled")
                .setMessage("Enable storage access so Firmium can show music saved to Music/Firmium.")
                .setPositiveButton("Open Settings") { _, _ ->
                    startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                        data = Uri.fromParts("package", packageName, null)
                    })
                }
                .setNegativeButton("Not now", null)
                .show()
        }
    }

    private var storagePermissionRequested = false

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)

        val filter = IntentFilter().apply {
            addAction(NowPlayingController.actionPrev(packageName))
            addAction(NowPlayingController.actionPlayPause(packageName))
            addAction(NowPlayingController.actionNext(packageName))
            addAction(NowPlayingController.actionDismiss(packageName))
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(mediaReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            @Suppress("UnspecifiedRegisterReceiverFlag")
            registerReceiver(mediaReceiver, filter)
        }

        setContent {
            // Observe theme and UI theme from DataStore — recompose the entire tree when they change.
            val themeId by app.prefs.themeId.collectAsStateWithLifecycle(initialValue = "firmium")

            FirmiumTheme(themeId = themeId) {
                val authViewModel: AuthViewModel = viewModel()
                val authState by authViewModel.state.collectAsStateWithLifecycle()

                if (!authState.isLoading) {
                    val playerViewModel: PlayerViewModel = viewModel()
                    val libraryViewModel: LibraryViewModel = viewModel()
                    val searchViewModel: SearchViewModel = viewModel()
                    val playlistViewModel: PlaylistViewModel = viewModel()
                    val scope = rememberCoroutineScope()
                    var showAccountDialog by remember { mutableStateOf(false) }

                    // Auto-open login dialog when saved credentials couldn't be restored.
                    LaunchedEffect(authState.needsLogin) {
                        if (authState.needsLogin) showAccountDialog = true
                    }

                    AppNavGraph(
                        auth = app.auth,
                        authViewModel = authViewModel,
                        playerViewModel = playerViewModel,
                        libraryViewModel = libraryViewModel,
                        searchViewModel = searchViewModel,
                        playlistViewModel = playlistViewModel,
                        currentThemeId = themeId,
                        onThemeSelected = { id -> scope.launch { app.prefs.setThemeId(id) } },
                        onAccountClick = { showAccountDialog = true },
                    )

                    if (showAccountDialog) {
                        AccountDialog(
                            state = authState,
                            isAuthenticated = authState.isAuthenticated,
                            serverUrl = app.auth.credentials?.server,
                            onLogin = { server, user, pass, savePass ->
                                authViewModel.login(server, user, pass, savePass)
                            },
                            onDisconnect = {
                                authViewModel.logout()
                                libraryViewModel.invalidateAll()
                                showAccountDialog = false
                            },
                            onDismiss = { showAccountDialog = false },
                        )
                    }

                    // Close the dialog automatically once a login succeeds and refresh the library.
                    LaunchedEffect(authState.isAuthenticated) {
                        if (authState.isAuthenticated && showAccountDialog) {
                            libraryViewModel.invalidateAll()
                            showAccountDialog = false
                        }
                    }
                }
            }
        }
    }

    override fun onResume() {
        super.onResume()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU && !notificationPermissionRequested) {
            notificationPermissionRequested = true
            if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS)
                != PackageManager.PERMISSION_GRANTED) {
                notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
        }
        if (!storagePermissionRequested) {
            storagePermissionRequested = true
            if (ContextCompat.checkSelfPermission(this, storagePermission)
                != PackageManager.PERMISSION_GRANTED) {
                storagePermissionLauncher.launch(storagePermission)
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        unregisterReceiver(mediaReceiver)
    }
}
