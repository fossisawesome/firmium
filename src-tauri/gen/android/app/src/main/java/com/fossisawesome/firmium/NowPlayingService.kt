package com.fossisawesome.firmium

import android.app.Notification
import android.app.Service
import android.content.Intent
import android.os.IBinder

// Foreground service that keeps the media notification alive when the app is backgrounded.
// Started by NowPlayingPlugin.updateNowPlaying; the plugin stores the notification in
// pendingNotification before calling startForegroundService so we can promote ourselves
// immediately in onStartCommand and avoid the 5-second foreground timeout crash.
class NowPlayingService : Service() {

    companion object {
        // NowPlayingPlugin writes here before calling startForegroundService().
        var pendingNotification: Notification? = null
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val notification = pendingNotification
        if (notification != null) {
            startForeground(NowPlayingPlugin.NOTIFICATION_ID, notification)
        } else {
            // No notification ready — stop immediately to avoid the foreground timeout crash.
            stopSelf()
        }
        return START_STICKY
    }
}
