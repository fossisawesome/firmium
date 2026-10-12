package com.fossisawesome.firmium.audio

import android.app.Notification
import android.app.Service
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioManager
import android.os.IBinder
import androidx.core.content.ContextCompat
import com.fossisawesome.firmium.FirmiumApplication

// Foreground service that keeps the Now Playing notification alive when the app is backgrounded.
// Uses the pendingNotification static set by NowPlayingController to avoid the 5s ANR timeout.
class NowPlayingService : Service() {

    companion object {
        // Passed in the Intent so rapid track skips cannot race on this field before onStartCommand
        // reads it. The field is kept as fallback for updatePosition/updatePlaybackState callers
        // that notify without restarting the service.
        const val EXTRA_NOTIFICATION = "notification"
        @Volatile
        var pendingNotification: Notification? = null
    }

    private var noisyReceiver: NoisyAudioReceiver? = null

    override fun onCreate() {
        super.onCreate()
        val controller = (application as FirmiumApplication).playback
        noisyReceiver = NoisyAudioReceiver(controller)
        ContextCompat.registerReceiver(
            this,
            noisyReceiver,
            IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY),
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )
    }

    override fun onDestroy() {
        noisyReceiver?.let { unregisterReceiver(it) }
        noisyReceiver = null
        super.onDestroy()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        @Suppress("DEPRECATION")
        val notification = intent?.getParcelableExtra<Notification>(EXTRA_NOTIFICATION)
            ?: pendingNotification
            ?: return START_NOT_STICKY
        startForeground(NOTIFICATION_ID, notification)
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
