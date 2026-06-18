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
import com.fossisawesome.firmium.ui.screens.OnboardingScreen
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

    private val storagePermission: String =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) Manifest.permission.READ_MEDIA_AUDIO
        else Manifest.permission.READ_EXTERNAL_STORAGE

    // Request notifications, microphone (visualizer), and audio/storage (local music) together.
    // A single RequestMultiplePermissions flow is required: launching several single-permission
    // requests back-to-back drops all but the first dialog, so only notifications ever showed.
    private val permissionsLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { results ->
        if (results[storagePermission] == true) app.localLibrary.invalidate()
        if (results[storagePermission] == false && !shouldShowRequestPermissionRationale(storagePermission)) {
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

    private var permissionsRequested = false

    // Notifications (media controls), microphone (visualizer), and audio storage (local library).
    private fun permissionsToRequest(): List<String> {
        val perms = mutableListOf<String>()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            perms.add(Manifest.permission.POST_NOTIFICATIONS)
        }
        perms.add(storagePermission)
        perms.add(Manifest.permission.RECORD_AUDIO)
        return perms
    }

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
                val onboarded by app.prefs.onboarded.collectAsStateWithLifecycle(initialValue = true)
                val serverUrl by app.prefs.serverUrl.collectAsStateWithLifecycle(initialValue = null)
                val onboardScope = rememberCoroutineScope()

                if (!onboarded && serverUrl == null) {
                    OnboardingScreen(onFinish = {
                        onboardScope.launch { app.prefs.setOnboarded(true) }
                    })
                } else if (!authState.isLoading) {
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
        if (!permissionsRequested) {
            permissionsRequested = true
            val needed = permissionsToRequest().filter {
                ContextCompat.checkSelfPermission(this, it) != PackageManager.PERMISSION_GRANTED
            }
            if (needed.isNotEmpty()) permissionsLauncher.launch(needed.toTypedArray())
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        unregisterReceiver(mediaReceiver)
    }
}
