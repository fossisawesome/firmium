package com.fossisawesome.firmium.audio

import android.app.Notification
import android.app.Service
import android.content.Intent
import android.os.IBinder

// Foreground service that keeps the Now Playing notification alive when the app is backgrounded.
// Uses the pendingNotification static set by NowPlayingController to avoid the 5s ANR timeout.
class NowPlayingService : Service() {

    companion object {
        // Set before startForegroundService() so onStartCommand can immediately call startForeground().
        @Volatile
        var pendingNotification: Notification? = null
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val notification = pendingNotification
            ?: return START_NOT_STICKY
        startForeground(NOTIFICATION_ID, notification)
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
