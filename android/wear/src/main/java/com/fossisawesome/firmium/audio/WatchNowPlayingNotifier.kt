package com.fossisawesome.firmium.audio

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationCompat
import com.fossisawesome.firmium.wear.MainActivity

private const val CHANNEL_ID = "firmium_watch_now_playing"
const val WATCH_NOTIFICATION_ID = 1

// Plain foreground-service notification for watch playback — title/artist only, single
// play-pause toggle action, no MediaSession/MediaStyle/seekbar/rating/queue/shuffle/repeat
// icons or album art. Full media-session/tile integration is a future increment once
// sub-project 4's now-playing screen exists to justify it.
class WatchNowPlayingNotifier(private val context: Context) {

    companion object {
        fun actionTogglePlayPause(pkg: String) = "$pkg.ACTION_TOGGLE_PLAY_PAUSE"
    }

    private val notificationManager =
        context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

    private fun ensureChannel() {
        val channel = NotificationChannel(CHANNEL_ID, "Now Playing", NotificationManager.IMPORTANCE_LOW).apply {
            description = "Firmium watch playback controls"
            setShowBadge(false)
        }
        notificationManager.createNotificationChannel(channel)
    }

    private fun pendingToggleBroadcast(): PendingIntent {
        val action = actionTogglePlayPause(context.packageName)
        val intent = Intent(action).setPackage(context.packageName)
        return PendingIntent.getBroadcast(
            context, action.hashCode(), intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    private fun openAppIntent(): PendingIntent {
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        return PendingIntent.getActivity(
            context, 0, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    private fun buildNotification(title: String, artist: String, isPlaying: Boolean): Notification {
        val playPauseIcon = if (isPlaying) android.R.drawable.ic_media_pause else android.R.drawable.ic_media_play
        return NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_media_play)
            .setContentTitle(title)
            .setContentText(artist)
            .setContentIntent(openAppIntent())
            .addAction(playPauseIcon, if (isPlaying) "Pause" else "Play", pendingToggleBroadcast())
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setOnlyAlertOnce(true)
            .setOngoing(isPlaying)
            .build()
    }

    fun update(title: String, artist: String, isPlaying: Boolean) {
        ensureChannel()
        val notification = buildNotification(title, artist, isPlaying)
        WatchNowPlayingService.pendingNotification = notification
        val serviceIntent = Intent(context, WatchNowPlayingService::class.java).apply {
            putExtra(WatchNowPlayingService.EXTRA_NOTIFICATION, notification)
        }
        try {
            context.startForegroundService(serviceIntent)
        } catch (_: IllegalStateException) {
            // App moved to background before this call — service cannot start. Non-fatal;
            // playback continues but without a notification until the app resumes.
        }
    }

    fun clear() {
        notificationManager.cancel(WATCH_NOTIFICATION_ID)
        context.stopService(Intent(context, WatchNowPlayingService::class.java))
    }
}
