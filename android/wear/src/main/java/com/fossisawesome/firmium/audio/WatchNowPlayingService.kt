package com.fossisawesome.firmium.audio

import android.app.Notification
import android.app.Service
import android.content.Intent
import android.os.IBinder

// Foreground service that keeps the Now Playing notification alive when the watch app is
// backgrounded. Mirrors the phone's NowPlayingService.
class WatchNowPlayingService : Service() {

    companion object {
        const val EXTRA_NOTIFICATION = "notification"
        @Volatile
        var pendingNotification: Notification? = null
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        @Suppress("DEPRECATION")
        val notification = intent?.getParcelableExtra<Notification>(EXTRA_NOTIFICATION)
            ?: pendingNotification
            ?: return START_NOT_STICKY
        startForeground(WATCH_NOTIFICATION_ID, notification)
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
