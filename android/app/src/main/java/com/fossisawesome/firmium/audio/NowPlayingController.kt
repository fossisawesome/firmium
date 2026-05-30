package com.fossisawesome.firmium.audio

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.drawable.BitmapDrawable
import android.os.Build
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.media.app.NotificationCompat.MediaStyle
import coil.imageLoader
import coil.request.ImageRequest
import com.fossisawesome.firmium.MainActivity
import com.fossisawesome.firmium.R
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private const val CHANNEL_ID = "firmium_now_playing"
const val NOTIFICATION_ID = 1

// MediaSession + persistent media notification. Ported from NowPlayingPlugin.kt with Tauri removed.
// The PlayerViewModel drives this directly instead of JS calling tauri commands.
class NowPlayingController(private val context: Context) {

    // Callback for media button presses from notification or headset.
    interface Listener {
        fun onPlay()
        fun onPause()
        fun onNext()
        fun onPrevious()
        fun onSeekTo(posMs: Long) {}  // default no-op for backwards compat
    }

    var listener: Listener? = null

    private val scope = CoroutineScope(Dispatchers.Main)
    private var mediaSession: MediaSessionCompat? = null
    private val notificationManager =
        context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

    private val actionPrev = "${context.packageName}.ACTION_PREV"
    private val actionPlayPause = "${context.packageName}.ACTION_PLAY_PAUSE"
    private val actionNext = "${context.packageName}.ACTION_NEXT"
    private val actionDismiss = "${context.packageName}.ACTION_DISMISS"

    // Called from MainActivity's BroadcastReceiver to forward notification button taps.
    fun handleAction(action: String) {
        when (action) {
            "prev" -> listener?.onPrevious()
            "next" -> listener?.onNext()
            "togglePlayPause" -> {
                val state = mediaSession?.controller?.playbackState?.state
                if (state == PlaybackStateCompat.STATE_PLAYING) listener?.onPause()
                else listener?.onPlay()
            }
        }
    }

    private fun ensureChannel() {
        val channel = NotificationChannel(CHANNEL_ID, "Now Playing", NotificationManager.IMPORTANCE_LOW).apply {
            description = "Firmium media playback controls"
            setShowBadge(false)
        }
        notificationManager.createNotificationChannel(channel)
    }

    private fun ensureMediaSession(): MediaSessionCompat {
        return mediaSession ?: MediaSessionCompat(context, "FirmiumMediaSession").also { session ->
            session.setCallback(object : MediaSessionCompat.Callback() {
                override fun onPlay() { listener?.onPlay() }
                override fun onPause() { listener?.onPause() }
                override fun onSkipToNext() { listener?.onNext() }
                override fun onSkipToPrevious() { listener?.onPrevious() }
                override fun onSeekTo(pos: Long) { listener?.onSeekTo(pos) }
            })
            session.isActive = true
            mediaSession = session
        }
    }

    private fun pendingBroadcast(action: String): PendingIntent {
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

    private fun buildNotification(title: String, artist: String, isPlaying: Boolean, art: Bitmap?, positionMs: Long, durationMs: Long): Notification {
        val session = ensureMediaSession()
        val playPauseIcon = if (isPlaying) android.R.drawable.ic_media_pause else android.R.drawable.ic_media_play

        // Real position enables the seekable progress bar on the lock screen / notification shade.
        session.setPlaybackState(
            PlaybackStateCompat.Builder()
                .setState(
                    if (isPlaying) PlaybackStateCompat.STATE_PLAYING else PlaybackStateCompat.STATE_PAUSED,
                    positionMs,
                    if (isPlaying) 1f else 0f,
                )
                .setActions(
                    PlaybackStateCompat.ACTION_PLAY_PAUSE or
                    PlaybackStateCompat.ACTION_SKIP_TO_NEXT or
                    PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS or
                    PlaybackStateCompat.ACTION_SEEK_TO,
                )
                .build()
        )

        return NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentTitle(title)
            .setContentText(artist)
            .setLargeIcon(art)
            .setContentIntent(openAppIntent())
            .setDeleteIntent(pendingBroadcast(actionDismiss))
            .addAction(android.R.drawable.ic_media_previous, "Previous", pendingBroadcast(actionPrev))
            .addAction(playPauseIcon, if (isPlaying) "Pause" else "Play", pendingBroadcast(actionPlayPause))
            .addAction(android.R.drawable.ic_media_next, "Next", pendingBroadcast(actionNext))
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

    fun update(title: String, artist: String, album: String, coverUrl: String?, isPlaying: Boolean) {
        ensureChannel()
        scope.launch {
            // Use Coil to load album art — benefits from the app-wide disk/memory cache.
            val art: Bitmap? = if (!coverUrl.isNullOrBlank()) {
                withContext(Dispatchers.IO) {
                    runCatching {
                        val req = ImageRequest.Builder(context)
                            .data(coverUrl)
                            .allowHardware(false)
                            .build()
                        (context.imageLoader.execute(req).drawable as? BitmapDrawable)?.bitmap
                    }.getOrNull()
                }
            } else null

            val session = ensureMediaSession()
            session.setMetadata(
                MediaMetadataCompat.Builder()
                    .putString(MediaMetadataCompat.METADATA_KEY_TITLE, title)
                    .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, artist)
                    .putString(MediaMetadataCompat.METADATA_KEY_ALBUM, album)
                    .putBitmap(MediaMetadataCompat.METADATA_KEY_ART, art)
                    .build()
            )

            val notification = buildNotification(title, artist, isPlaying, art, 0L, 0L)
            NowPlayingService.pendingNotification = notification
            val serviceIntent = Intent(context, NowPlayingService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(serviceIntent)
            } else {
                context.startService(serviceIntent)
                NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, notification)
            }
        }
    }

    // Lightweight update called every 250ms during playback to drive the notification seekbar.
    fun updatePosition(positionMs: Long, durationMs: Long, isPlaying: Boolean) {
        val session = mediaSession ?: return
        val meta = session.controller?.metadata ?: return
        val title = meta.getString(MediaMetadataCompat.METADATA_KEY_TITLE) ?: return

        // Update duration in metadata if it changed.
        if (meta.getLong(MediaMetadataCompat.METADATA_KEY_DURATION) != durationMs) {
            session.setMetadata(
                MediaMetadataCompat.Builder(meta)
                    .putLong(MediaMetadataCompat.METADATA_KEY_DURATION, durationMs)
                    .build()
            )
        }

        val artist = meta.getString(MediaMetadataCompat.METADATA_KEY_ARTIST) ?: ""
        val art = meta.getBitmap(MediaMetadataCompat.METADATA_KEY_ART)
        val notification = buildNotification(title, artist, isPlaying, art, positionMs, durationMs)
        NowPlayingService.pendingNotification = notification
        NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, notification)
    }

    fun updatePlaybackState(isPlaying: Boolean) {
        val session = mediaSession ?: return
        val meta = session.controller?.metadata
        val art = meta?.getBitmap(MediaMetadataCompat.METADATA_KEY_ART)
        val title = meta?.getString(MediaMetadataCompat.METADATA_KEY_TITLE) ?: return
        val artist = meta.getString(MediaMetadataCompat.METADATA_KEY_ARTIST) ?: ""
        val notification = buildNotification(title, artist, isPlaying, art, PlaybackStateCompat.PLAYBACK_POSITION_UNKNOWN, 0L)
        NowPlayingService.pendingNotification = notification
        NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, notification)
    }

    fun clear() {
        NotificationManagerCompat.from(context).cancel(NOTIFICATION_ID)
        context.stopService(Intent(context, NowPlayingService::class.java))
        mediaSession?.release()
        mediaSession = null
    }

    // Companion holds the package-level broadcast action names for MainActivity.
    companion object {
        fun actionPrev(pkg: String) = "$pkg.ACTION_PREV"
        fun actionPlayPause(pkg: String) = "$pkg.ACTION_PLAY_PAUSE"
        fun actionNext(pkg: String) = "$pkg.ACTION_NEXT"
        fun actionDismiss(pkg: String) = "$pkg.ACTION_DISMISS"
    }
}
