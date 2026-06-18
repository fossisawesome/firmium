package com.fossisawesome.firmium.audio

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.drawable.BitmapDrawable
import android.net.Uri
import android.os.Build
import android.support.v4.media.MediaDescriptionCompat
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.media.app.NotificationCompat.MediaStyle
import androidx.palette.graphics.Palette
import coil.imageLoader
import coil.request.ImageRequest
import com.fossisawesome.firmium.MainActivity
import com.fossisawesome.firmium.R
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
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
        // Android Auto browse/voice playback — default no-op for backwards compat.
        fun onPlayFromMediaId(mediaId: String) {}
        fun onPlayFromSearch(query: String) {}
        // Android Auto queue list + shuffle/repeat toggles.
        fun onSkipToQueueItem(index: Long) {}
        fun onSetShuffleMode(enabled: Boolean) {}
        fun onSetRepeatMode(repeatMode: String) {}
    }

    var listener: Listener? = null

    private val scope = CoroutineScope(Dispatchers.Main)
    // Tracks the current art-fetch coroutine so it can be cancelled on track change or clear().
    private var artJob: Job? = null
    // Last position/duration reported by updatePosition(); reused by updatePlaybackState()
    // so the paused notification keeps the real elapsed time instead of 0:00.
    private var lastPositionMs: Long = 0L
    private var lastDurationMs: Long = 0L
    // Dominant cover-art color, used to tint the notification and the Android Auto UI.
    private var accentColor: Int? = null
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

    // Exposes the shared media session (creating it on demand) so FirmiumMediaBrowserService can
    // publish its token to Android Auto in onCreate, before any track has played.
    fun session(): MediaSessionCompat = ensureMediaSession()

    private fun ensureMediaSession(): MediaSessionCompat {
        return mediaSession ?: MediaSessionCompat(context, "FirmiumMediaSession").also { session ->
            session.setCallback(object : MediaSessionCompat.Callback() {
                override fun onPlay() { listener?.onPlay() }
                override fun onPause() { listener?.onPause() }
                override fun onSkipToNext() { listener?.onNext() }
                override fun onSkipToPrevious() { listener?.onPrevious() }
                override fun onSeekTo(pos: Long) { listener?.onSeekTo(pos) }
                override fun onPlayFromMediaId(mediaId: String?, extras: android.os.Bundle?) {
                    mediaId?.let { listener?.onPlayFromMediaId(it) }
                }
                override fun onPrepareFromMediaId(mediaId: String?, extras: android.os.Bundle?) {
                    mediaId?.let { listener?.onPlayFromMediaId(it) }
                }
                override fun onPlayFromSearch(query: String?, extras: android.os.Bundle?) {
                    listener?.onPlayFromSearch(query ?: "")
                }
                override fun onSkipToQueueItem(id: Long) { listener?.onSkipToQueueItem(id) }
                override fun onSetShuffleMode(shuffleMode: Int) {
                    listener?.onSetShuffleMode(shuffleMode != PlaybackStateCompat.SHUFFLE_MODE_NONE)
                }
                override fun onSetRepeatMode(repeatMode: Int) {
                    listener?.onSetRepeatMode(when (repeatMode) {
                        PlaybackStateCompat.REPEAT_MODE_ONE -> "one"
                        PlaybackStateCompat.REPEAT_MODE_ALL, PlaybackStateCompat.REPEAT_MODE_GROUP -> "all"
                        else -> "none"
                    })
                }
            })
            // Advertise the browse/voice play actions on an idle state so Android Auto can start
            // playback from cold (no track loaded yet); buildNotification() overwrites this once
            // a track is playing.
            session.setPlaybackState(
                PlaybackStateCompat.Builder()
                    .setState(PlaybackStateCompat.STATE_NONE, 0L, 0f)
                    .setActions(
                        PlaybackStateCompat.ACTION_PLAY or
                        PlaybackStateCompat.ACTION_PLAY_PAUSE or
                        PlaybackStateCompat.ACTION_PLAY_FROM_MEDIA_ID or
                        PlaybackStateCompat.ACTION_PLAY_FROM_SEARCH or
                        PlaybackStateCompat.ACTION_PREPARE_FROM_MEDIA_ID or
                        PlaybackStateCompat.ACTION_SKIP_TO_NEXT or
                        PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS or
                        PlaybackStateCompat.ACTION_SEEK_TO
                    )
                    .build()
            )
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

    // Builds a MediaMetadataCompat with common fields; art is optional.
    private fun buildMetadata(title: String, artist: String, album: String, art: Bitmap? = null): MediaMetadataCompat =
        MediaMetadataCompat.Builder()
            .putString(MediaMetadataCompat.METADATA_KEY_TITLE, title)
            .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, artist)
            .putString(MediaMetadataCompat.METADATA_KEY_ALBUM, album)
            .apply { if (art != null) putBitmap(MediaMetadataCompat.METADATA_KEY_ART, art) }
            .build()

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
                    PlaybackStateCompat.ACTION_PLAY or
                    PlaybackStateCompat.ACTION_PAUSE or
                    PlaybackStateCompat.ACTION_PLAY_PAUSE or
                    PlaybackStateCompat.ACTION_SKIP_TO_NEXT or
                    PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS or
                    PlaybackStateCompat.ACTION_SKIP_TO_QUEUE_ITEM or
                    PlaybackStateCompat.ACTION_SET_SHUFFLE_MODE or
                    PlaybackStateCompat.ACTION_SET_REPEAT_MODE or
                    PlaybackStateCompat.ACTION_SEEK_TO,
                )
                .build()
        )

        return NotificationCompat.Builder(context, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_firmium)
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
            .apply { accentColor?.let { setColor(it); setColorized(true) } }
            .build()
    }

    fun update(title: String, artist: String, album: String, coverUrl: String?, isPlaying: Boolean) {
        ensureChannel()
        val session = ensureMediaSession()
        // Reset cached position so a pause right after a track starts reads 0, not the
        // previous track's elapsed time.
        lastPositionMs = 0L
        lastDurationMs = 0L
        accentColor = null

        // Update metadata immediately (no art yet) so the session reflects the new track at once.
        session.setMetadata(buildMetadata(title, artist, album))

        // Start the foreground service synchronously so it fires before the app can move to the
        // background. Deferring this behind the async art fetch caused an IllegalStateException
        // on Android O+ ("Not allowed to start service; app is in background").
        val noArtNotification = buildNotification(title, artist, isPlaying, null, 0L, 0L)
        NowPlayingService.pendingNotification = noArtNotification
        // Pass the notification in the Intent itself so rapid track skips cannot overwrite the
        // static pendingNotification field before onStartCommand reads it.
        val serviceIntent = Intent(context, NowPlayingService::class.java).apply {
            putExtra(NowPlayingService.EXTRA_NOTIFICATION, noArtNotification)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            try {
                context.startForegroundService(serviceIntent)
            } catch (_: IllegalStateException) {
                // App moved to background before this call — service cannot start. Non-fatal;
                // playback continues but without a media notification until the app resumes.
                return
            }
        } else {
            try {
                context.startService(serviceIntent)
            } catch (_: Exception) {
                // Some restricted OEM builds (pre-O) enforce background start rules too.
                return
            }
            NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, noArtNotification)
        }

        // Cancel any in-flight art fetch for the previous track before launching a new one.
        artJob?.cancel()

        // Fetch album art asynchronously and refresh the notification once it arrives.
        if (!coverUrl.isNullOrBlank()) {
            artJob = scope.launch {
                val art: Bitmap? = withContext(Dispatchers.IO) {
                    runCatching {
                        val req = ImageRequest.Builder(context)
                            .data(coverUrl)
                            .allowHardware(false)
                            .build()
                        (context.imageLoader.execute(req).drawable as? BitmapDrawable)?.bitmap
                    }.getOrNull()
                }
                // Guard against clear() having been called (mediaSession released) or a newer
                // track having started while art was loading.
                if (session !== mediaSession) return@launch
                if (art != null) {
                    // Pull a dominant color so the notification + Android Auto tint to the cover.
                    accentColor = runCatching {
                        Palette.from(art).generate().let { it.vibrantSwatch ?: it.dominantSwatch }?.rgb
                    }.getOrNull()
                    session.setMetadata(buildMetadata(title, artist, album, art))
                    val artNotification = buildNotification(title, artist, isPlaying, art, 0L, 0L)
                    NowPlayingService.pendingNotification = artNotification
                    // Re-promote the foreground service with the art notification so it stays
                    // alive even if Android killed it while the fetch was in progress.
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                        val artIntent = Intent(context, NowPlayingService::class.java).apply {
                            putExtra(NowPlayingService.EXTRA_NOTIFICATION, artNotification)
                        }
                        try {
                            context.startForegroundService(artIntent)
                        } catch (_: IllegalStateException) {
                            NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, artNotification)
                        }
                    } else {
                        NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, artNotification)
                    }
                }
            }
        }
    }

    // Lightweight update called every 250ms during playback to drive the notification seekbar.
    fun updatePosition(positionMs: Long, durationMs: Long, isPlaying: Boolean) {
        val session = mediaSession ?: return
        val meta = session.controller?.metadata ?: return
        val title = meta.getString(MediaMetadataCompat.METADATA_KEY_TITLE) ?: return

        lastPositionMs = positionMs
        lastDurationMs = durationMs

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
        val notification = buildNotification(title, artist, isPlaying, art, lastPositionMs, lastDurationMs)
        NowPlayingService.pendingNotification = notification
        NotificationManagerCompat.from(context).notify(NOTIFICATION_ID, notification)
    }

    // One entry of the Android Auto / lock-screen queue list.
    data class QueueEntry(val id: String, val title: String, val artist: String, val coverUrl: String?)

    // Publishes the queue so Android Auto shows an "Up Next" list and onSkipToQueueItem works.
    fun setQueue(entries: List<QueueEntry>) {
        val session = mediaSession ?: return
        val items = entries.mapIndexed { i, e ->
            val desc = MediaDescriptionCompat.Builder()
                .setMediaId(e.id)
                .setTitle(e.title)
                .setSubtitle(e.artist)
                .apply { e.coverUrl?.let { setIconUri(Uri.parse(it)) } }
                .build()
            MediaSessionCompat.QueueItem(desc, i.toLong())
        }
        session.setQueue(items)
        session.setQueueTitle("Up Next")
    }

    fun setShuffleMode(enabled: Boolean) {
        mediaSession?.setShuffleMode(
            if (enabled) PlaybackStateCompat.SHUFFLE_MODE_ALL else PlaybackStateCompat.SHUFFLE_MODE_NONE
        )
    }

    fun setRepeatMode(mode: String) {
        mediaSession?.setRepeatMode(when (mode) {
            "one" -> PlaybackStateCompat.REPEAT_MODE_ONE
            "all" -> PlaybackStateCompat.REPEAT_MODE_ALL
            else -> PlaybackStateCompat.REPEAT_MODE_NONE
        })
    }

    fun clear() {
        // Cancel in-flight art fetch before releasing the session so the coroutine cannot
        // call setMetadata() or notify() on a dead session after this returns.
        artJob?.cancel()
        artJob = null
        lastPositionMs = 0L
        lastDurationMs = 0L
        accentColor = null
        mediaSession?.setQueue(null)
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
