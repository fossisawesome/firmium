package com.fossisawesome.firmium.audio

import android.content.Context
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.common.TrackSelectionParameters.AudioOffloadPreferences
import kotlinx.coroutines.*
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlin.math.pow

// Native ExoPlayer-based audio engine. Ported from AudioPlugin.kt with Tauri removed.
// All methods are main-thread-safe; callers use the exposed suspend functions or callbacks.
class AudioPlayer(private val context: Context) {

    // Callback interface replaces Tauri plugin event emitters.
    interface Listener {
        fun onStateChanged(playerId: String, state: String)
        fun onTrackChanged(playerId: String, trackId: String, index: Int)
        fun onPlaybackFinished(playerId: String)
    }

    var listener: Listener? = null
    var bitPerfectMode: String = "off"

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private val sessions = ConcurrentHashMap<String, AudioSession>()

    private val audioAttrs = AudioAttributes.Builder()
        .setUsage(C.USAGE_MEDIA)
        .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
        .build()

    private fun buildPlayer(): ExoPlayer {
        val player = ExoPlayer.Builder(context)
            .setAudioAttributes(audioAttrs, true)
            .build()
        when (bitPerfectMode) {
            "strict" -> {
                player.trackSelectionParameters = player.trackSelectionParameters
                    .buildUpon()
                    .setAudioOffloadPreferences(
                        AudioOffloadPreferences.Builder()
                            .setAudioOffloadMode(AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_REQUIRED)
                            .setIsGaplessSupportRequired(true)
                            .build()
                    )
                    .build()
            }
            "relaxed" -> {
                player.trackSelectionParameters = player.trackSelectionParameters
                    .buildUpon()
                    .setAudioOffloadPreferences(
                        AudioOffloadPreferences.Builder()
                            .setAudioOffloadMode(AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_ENABLED)
                            .setIsGaplessSupportRequired(false)
                            .build()
                    )
                    .build()
            }
            // "off" — no offload config; standard ExoPlayer software pipeline
        }
        return player
    }

    private fun gainFactor(gainDb: Float?): Float =
        if (gainDb != null) (10.0.pow(gainDb / 20.0)).toFloat().coerceIn(0.01f, 4.0f)
        else 1.0f

    private fun attachListeners(playerId: String, session: AudioSession) {
        session.player.addListener(object : Player.Listener {
            override fun onIsPlayingChanged(isPlaying: Boolean) {
                val state = if (isPlaying) "playing"
                    else if (session.player.playbackState == Player.STATE_READY) "paused"
                    else return
                listener?.onStateChanged(playerId, state)
            }

            override fun onPlaybackStateChanged(state: Int) {
                when (state) {
                    Player.STATE_BUFFERING -> listener?.onStateChanged(playerId, "loading")
                    Player.STATE_IDLE -> listener?.onStateChanged(playerId, "stopped")
                    else -> {}
                }
            }

            override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
                val queueIds = session.queueTrackIds ?: return
                val idx = session.player.currentMediaItemIndex
                val newTrackId = queueIds.getOrNull(idx) ?: return
                session.currentTrackId = newTrackId
                session.currentQueueIndex = idx
                val newGain = session.queueReplayGainFactors?.getOrNull(idx) ?: 1.0f
                session.replayGainFactor = newGain
                session.player.volume = session.baseVolume * newGain
                listener?.onTrackChanged(playerId, newTrackId, idx)
            }
        })

        val job = scope.launch {
            while (sessions.containsKey(playerId)) {
                delay(100)
                val s = sessions[playerId] ?: break
                if (s.player.playbackState == Player.STATE_ENDED) {
                    sessions.remove(playerId)
                    s.player.release()
                    listener?.onPlaybackFinished(playerId)
                    break
                }
            }
        }
        session.finishWatchJob = job
    }

    private fun releaseSession(playerId: String) {
        sessions.remove(playerId)?.let { s ->
            s.finishWatchJob?.cancel()
            s.fadeJob?.cancel()
            s.player.stop()
            s.player.release()
        }
    }

    // ── Playback commands ──────────────────────────────────────────────────────

    fun play(streamUrl: String, trackId: String, replayGainDb: Float? = null): String {
        val playerId = UUID.randomUUID().toString()
        val player = buildPlayer()
        val gain = gainFactor(replayGainDb)
        player.volume = gain
        val session = AudioSession(player, trackId, 1.0f, gain)
        sessions[playerId] = session
        attachListeners(playerId, session)
        player.setMediaItem(MediaItem.fromUri(streamUrl))
        player.prepare()
        player.playWhenReady = true
        return playerId
    }

    fun resume(playerId: String) {
        val session = sessions[playerId] ?: return
        if (session.finishWatchJob == null) attachListeners(playerId, session)
        session.player.play()
    }

    fun pause(playerId: String) {
        val session = sessions[playerId] ?: return
        session.fadeJob?.cancel()
        session.fadeJob = scope.launch {
            val vol = session.player.volume
            repeat(5) { i ->
                session.player.volume = vol * (1f - (i + 1) / 5f)
                delay(4)
            }
            session.player.pause()
            session.player.volume = vol
        }
    }

    fun stop(playerId: String) {
        val session = sessions[playerId] ?: return
        session.fadeJob?.cancel()
        session.fadeJob = scope.launch {
            val vol = session.player.volume
            repeat(5) { i ->
                session.player.volume = vol * (1f - (i + 1) / 5f)
                delay(4)
            }
            releaseSession(playerId)
        }
    }

    fun stopAll() {
        sessions.keys.toList().forEach { releaseSession(it) }
    }

    fun seek(playerId: String, positionSeconds: Double) {
        sessions[playerId]?.player?.seekTo((positionSeconds * 1000).toLong())
    }

    fun setVolume(playerId: String, volume: Float) {
        val session = sessions[playerId] ?: return
        session.baseVolume = volume.coerceIn(0f, 1f)
        session.player.volume = session.baseVolume * session.replayGainFactor
    }

    fun getVolume(playerId: String): Float = sessions[playerId]?.baseVolume ?: 1f

    fun getState(playerId: String): String {
        val session = sessions[playerId] ?: return "stopped"
        return when {
            session.player.playbackState == Player.STATE_BUFFERING -> "loading"
            session.player.isPlaying -> "playing"
            session.player.playbackState == Player.STATE_ENDED -> "stopped"
            else -> "paused"
        }
    }

    fun getPosition(playerId: String): Double =
        (sessions[playerId]?.player?.currentPosition ?: 0L) / 1000.0

    fun getDuration(playerId: String): Double? {
        val dur = sessions[playerId]?.player?.duration ?: return null
        return if (dur != C.TIME_UNSET && dur > 0) dur / 1000.0 else null
    }

    fun getAudioSessionId(playerId: String): Int =
        sessions[playerId]?.player?.audioSessionId ?: 0

    fun isFinished(playerId: String): Boolean {
        val session = sessions[playerId] ?: return true
        return session.player.playbackState == Player.STATE_ENDED
    }

    // ── Queue mode (single ExoPlayer with full playlist) ──────────────────────

    fun setQueue(
        tracks: List<QueueTrack>,
        startIndex: Int,
        volume: Float,
    ): String {
        sessions.keys.toList().forEach { releaseSession(it) }
        val playerId = UUID.randomUUID().toString()
        val player = buildPlayer()
        val gainFactors = tracks.map { gainFactor(it.replayGainDb) }
        val idx = startIndex.coerceIn(0, (tracks.size - 1).coerceAtLeast(0))
        val initialGain = gainFactors.getOrElse(idx) { 1.0f }
        player.volume = volume * initialGain

        val session = AudioSession(
            player = player,
            currentTrackId = tracks.getOrNull(idx)?.trackId ?: "",
            baseVolume = volume,
            replayGainFactor = initialGain,
            queueTrackIds = tracks.map { it.trackId },
            queueReplayGainFactors = gainFactors,
            currentQueueIndex = idx,
        )
        sessions[playerId] = session
        attachListeners(playerId, session)

        player.setMediaItems(tracks.map { MediaItem.fromUri(it.streamUrl) }, idx, 0L)
        player.prepare()
        player.playWhenReady = true
        return playerId
    }

    fun skipToNext(playerId: String) {
        sessions[playerId]?.player?.seekToNextMediaItem()
    }

    fun skipToPrevious(playerId: String) {
        sessions[playerId]?.player?.seekToPreviousMediaItem()
    }

    fun skipToIndex(playerId: String, index: Int) {
        sessions[playerId]?.player?.seekTo(index, 0L)
    }

    fun getQueueIndex(playerId: String): Pair<Int, String>? {
        val s = sessions[playerId] ?: return null
        return s.currentQueueIndex to s.currentTrackId
    }

    // ── Crossfade ──────────────────────────────────────────────────────────────

    fun crossfadeTo(
        oldPlayerId: String,
        streamUrl: String,
        trackId: String,
        fadeDurationMs: Long,
        targetVolume: Float,
        replayGainDb: Float? = null,
    ): String {
        val newPlayerId = UUID.randomUUID().toString()
        val newPlayer = buildPlayer()
        val gain = gainFactor(replayGainDb)
        newPlayer.volume = 0f
        val newSession = AudioSession(newPlayer, trackId, targetVolume, gain)
        sessions[newPlayerId] = newSession
        attachListeners(newPlayerId, newSession)
        newPlayer.setMediaItem(MediaItem.fromUri(streamUrl))
        newPlayer.prepare()
        newPlayer.playWhenReady = true

        val steps = 25
        scope.launch {
            val stepMs = (fadeDurationMs / steps).coerceAtLeast(50)
            repeat(steps) { step ->
                delay(stepMs)
                val progress = (step + 1).toFloat() / steps
                sessions[oldPlayerId]?.player?.volume = (targetVolume * (1f - progress)).coerceAtLeast(0f)
                sessions[newPlayerId]?.player?.volume = targetVolume * progress * gain
            }
            releaseSession(oldPlayerId)
        }
        return newPlayerId
    }

    fun release() {
        scope.cancel()
        sessions.keys.toList().forEach { releaseSession(it) }
    }
}

// Track entry for queue mode.
data class QueueTrack(val streamUrl: String, val trackId: String, val replayGainDb: Float? = null)

private data class AudioSession(
    val player: ExoPlayer,
    var currentTrackId: String,
    var baseVolume: Float = 1.0f,
    var replayGainFactor: Float = 1.0f,
    var finishWatchJob: Job? = null,
    var fadeJob: Job? = null,
    val queueTrackIds: List<String>? = null,
    val queueReplayGainFactors: List<Float>? = null,
    var currentQueueIndex: Int = 0,
)
