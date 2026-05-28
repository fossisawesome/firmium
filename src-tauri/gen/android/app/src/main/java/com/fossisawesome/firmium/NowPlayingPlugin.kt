package com.fossisawesome.firmium

import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.os.Build
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.media.app.NotificationCompat.MediaStyle
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.net.URL

private const val CHANNEL_ID = "firmium_now_playing"

@InvokeArg
class NowPlayingArgs {
    var title: String = ""
    var artist: String = ""
    var album: String = ""
    var coverUrl: String = ""
    var isPlaying: Boolean = false
}

@InvokeArg
class PlaybackStateArgs {
    var isPlaying: Boolean = false
}

// Posts a MediaStyle notification so the user can control playback from
// the lock screen / notification shade when Firmium is backgrounded.
@TauriPlugin
class NowPlayingPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        // Held so MainActivity's BroadcastReceiver can forward button taps.
        var instance: NowPlayingPlugin? = null
        // Shared with NowPlayingService so it can call startForeground immediately.
        const val NOTIFICATION_ID = 1
    }

    init { instance = this }

    private var mediaSession: MediaSessionCompat? = null
    private val notificationManager by lazy {
        activity.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    }
    private val scope = CoroutineScope(Dispatchers.Main)

    // Broadcast actions for notification buttons
    private val ACTION_PREV = "${activity.packageName}.ACTION_PREV"
    private val ACTION_PLAY_PAUSE = "${activity.packageName}.ACTION_PLAY_PAUSE"
    private val ACTION_NEXT = "${activity.packageName}.ACTION_NEXT"

    private fun ensureChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Now Playing",
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = "Firmium media playback controls"
            setShowBadge(false)
        }
        notificationManager.createNotificationChannel(channel)
    }

    private fun ensureMediaSession(): MediaSessionCompat {
        return mediaSession ?: MediaSessionCompat(activity, "FirmiumMediaSession").also { session ->
            session.setCallback(object : MediaSessionCompat.Callback() {
                override fun onPlay() { trigger("mediaAction", JSObject().apply { put("action", "play") }) }
                override fun onPause() { trigger("mediaAction", JSObject().apply { put("action", "pause") }) }
                override fun onSkipToNext() { trigger("mediaAction", JSObject().apply { put("action", "next") }) }
                override fun onSkipToPrevious() { trigger("mediaAction", JSObject().apply { put("action", "prev") }) }
            })
            session.isActive = true
            mediaSession = session
        }
    }

    private fun pendingIntent(action: String): PendingIntent {
        val intent = Intent(action).setPackage(activity.packageName)
        return PendingIntent.getBroadcast(
            activity, 0, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
    }

    private fun openAppIntent(): PendingIntent {
        val intent = Intent(activity, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        return PendingIntent.getActivity(
            activity, 0, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
    }

    private fun buildNotification(
        title: String,
        artist: String,
        isPlaying: Boolean,
        art: Bitmap?
    ): Notification {
        val session = ensureMediaSession()

        val playPauseIcon = if (isPlaying)
            android.R.drawable.ic_media_pause
        else
            android.R.drawable.ic_media_play
        val playPauseLabel = if (isPlaying) "Pause" else "Play"

        val state = if (isPlaying) PlaybackStateCompat.STATE_PLAYING else PlaybackStateCompat.STATE_PAUSED
        session.setPlaybackState(
            PlaybackStateCompat.Builder()
                .setState(state, PlaybackStateCompat.PLAYBACK_POSITION_UNKNOWN, 1f)
                .setActions(
                    PlaybackStateCompat.ACTION_PLAY_PAUSE or
                    PlaybackStateCompat.ACTION_SKIP_TO_NEXT or
                    PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS
                )
                .build()
        )

        return NotificationCompat.Builder(activity, CHANNEL_ID)
            .setSmallIcon(R.mipmap.ic_launcher_foreground)
            .setContentTitle(title)
            .setContentText(artist)
            .setLargeIcon(art)
            .setContentIntent(openAppIntent())
            .setDeleteIntent(pendingIntent("${activity.packageName}.ACTION_DISMISS"))
            .addAction(android.R.drawable.ic_media_previous, "Previous", pendingIntent(ACTION_PREV))
            .addAction(playPauseIcon, playPauseLabel, pendingIntent(ACTION_PLAY_PAUSE))
            .addAction(android.R.drawable.ic_media_next, "Next", pendingIntent(ACTION_NEXT))
            .setStyle(
                MediaStyle()
                    .setMediaSession(session.sessionToken)
                    .setShowActionsInCompactView(0, 1, 2)
            )
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setOnlyAlertOnce(true)
            .setOngoing(isPlaying)
            .build()
    }

    @Command
    fun updateNowPlaying(invoke: Invoke) {
        val args = invoke.parseArgs(NowPlayingArgs::class.java)
        ensureChannel()

        scope.launch {
            // Fetch cover art off the main thread
            val art: Bitmap? = if (args.coverUrl.isNotBlank()) {
                withContext(Dispatchers.IO) {
                    runCatching {
                        val conn = URL(args.coverUrl).openConnection()
                        conn.connectTimeout = 3000
                        conn.readTimeout = 3000
                        BitmapFactory.decodeStream(conn.getInputStream())
                    }.getOrNull()
                }
            } else null

            // Update MediaSession metadata
            ensureMediaSession().setMetadata(
                MediaMetadataCompat.Builder()
                    .putString(MediaMetadataCompat.METADATA_KEY_TITLE, args.title)
                    .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, args.artist)
                    .putString(MediaMetadataCompat.METADATA_KEY_ALBUM, args.album)
                    .putBitmap(MediaMetadataCompat.METADATA_KEY_ART, art)
                    .build()
            )

            val notification = buildNotification(args.title, args.artist, args.isPlaying, art)

            // Store before starting so the service can call startForeground() immediately
            // in onStartCommand, avoiding the 5-second foreground-service timeout crash.
            NowPlayingService.pendingNotification = notification
            val serviceIntent = Intent(activity, NowPlayingService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                activity.startForegroundService(serviceIntent)
            } else {
                activity.startService(serviceIntent)
                NotificationManagerCompat.from(activity).notify(NOTIFICATION_ID, notification)
            }
            invoke.resolve()
        }
    }

    @Command
    fun updatePlaybackState(invoke: Invoke) {
        val args = invoke.parseArgs(PlaybackStateArgs::class.java)
        val session = mediaSession ?: run { invoke.resolve(); return }
        val state = if (args.isPlaying) PlaybackStateCompat.STATE_PLAYING else PlaybackStateCompat.STATE_PAUSED
        session.setPlaybackState(
            PlaybackStateCompat.Builder()
                .setState(state, PlaybackStateCompat.PLAYBACK_POSITION_UNKNOWN, 1f)
                .setActions(
                    PlaybackStateCompat.ACTION_PLAY_PAUSE or
                    PlaybackStateCompat.ACTION_SKIP_TO_NEXT or
                    PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS
                )
                .build()
        )
        // Re-post notification so the play/pause icon updates
        val meta = session.controller?.metadata
        val art = meta?.getBitmap(MediaMetadataCompat.METADATA_KEY_ART)
        val title = meta?.getString(MediaMetadataCompat.METADATA_KEY_TITLE) ?: ""
        val artist = meta?.getString(MediaMetadataCompat.METADATA_KEY_ARTIST) ?: ""
        val notification = buildNotification(title, artist, args.isPlaying, art)
        NowPlayingService.pendingNotification = notification
        NotificationManagerCompat.from(activity).notify(NOTIFICATION_ID, notification)
        invoke.resolve()
    }

    @Command
    fun clearNowPlaying(invoke: Invoke) {
        clearNowPlayingInternal()
        invoke.resolve()
    }

    // Called internally (e.g. from broadcast receiver) without a Tauri Invoke context.
    fun clearNowPlayingInternal() {
        NotificationManagerCompat.from(activity).cancel(NOTIFICATION_ID)
        activity.stopService(Intent(activity, NowPlayingService::class.java))
        mediaSession?.release()
        mediaSession = null
    }
}
