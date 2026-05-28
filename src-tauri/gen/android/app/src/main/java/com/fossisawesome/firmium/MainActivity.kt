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
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.core.content.ContextCompat
import app.tauri.plugin.JSObject

class MainActivity : TauriActivity() {

  // Modern permission launcher — must be registered before onCreate completes.
  private val notificationPermissionLauncher = registerForActivityResult(
    ActivityResultContracts.RequestPermission()
  ) { isGranted ->
    if (!isGranted) {
      // If the system dialog was shown and denied, offer to open app settings.
      if (!shouldShowRequestPermissionRationale(Manifest.permission.POST_NOTIFICATIONS)) {
        AlertDialog.Builder(this)
          .setTitle("Notifications disabled")
          .setMessage("To see Now Playing controls on the lock screen, enable notifications for Firmium in Settings.")
          .setPositiveButton("Open Settings") { _, _ ->
            startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
              data = Uri.fromParts("package", packageName, null)
            })
          }
          .setNegativeButton("Not now", null)
          .show()
      }
    }
  }

  // True once we have requested the permission this session, so we don't re-prompt on every resume.
  private var notificationPermissionRequested = false

  // Receives broadcast intents from media notification buttons and forwards
  // them to NowPlayingPlugin as mediaAction events.
  private val mediaReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      val plugin = NowPlayingPlugin.instance ?: return
      val action = when (intent.action) {
        "${packageName}.ACTION_PREV"       -> "prev"
        "${packageName}.ACTION_PLAY_PAUSE" -> "togglePlayPause"
        "${packageName}.ACTION_NEXT"       -> "next"
        "${packageName}.ACTION_DISMISS"    -> { plugin.clearNowPlayingInternal(); return }
        else -> return
      }
      plugin.trigger("mediaAction", JSObject().apply { put("action", action) })
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    val filter = IntentFilter().apply {
      addAction("${packageName}.ACTION_PREV")
      addAction("${packageName}.ACTION_PLAY_PAUSE")
      addAction("${packageName}.ACTION_NEXT")
      addAction("${packageName}.ACTION_DISMISS")
    }
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
      registerReceiver(mediaReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
    } else {
      registerReceiver(mediaReceiver, filter)
    }
  }

  override fun onResume() {
    super.onResume()
    // Request POST_NOTIFICATIONS once the window is visible and interactive.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU && !notificationPermissionRequested) {
      notificationPermissionRequested = true
      if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS)
          != PackageManager.PERMISSION_GRANTED) {
        notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
      }
    }
  }

  override fun onDestroy() {
    super.onDestroy()
    unregisterReceiver(mediaReceiver)
  }
}
