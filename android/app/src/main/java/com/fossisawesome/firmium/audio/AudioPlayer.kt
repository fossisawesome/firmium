package com.fossisawesome.firmium.audio

import android.content.Context
import android.media.AudioManager
import android.util.Log
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.audio.AudioSink
import androidx.media3.exoplayer.audio.DefaultAudioSink
import kotlinx.coroutines.*
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlin.math.pow

private const val TAG = "FirmiumAudio"

// Native ExoPlayer-based audio engine. Ported from AudioPlugin.kt with Tauri removed.
// All methods are main-thread-safe; callers use the exposed suspend functions or callbacks.
class AudioPlayer(private val context: Context) {

    // Callback interface replaces Tauri plugin event emitters.
    interface Listener {
        fun onStateChanged(playerId: String, state: String)
        fun onTrackChanged(playerId: String, trackId: String, index: Int, previousTrackId: String?, wasNaturalCompletion: Boolean)
        fun onPlaybackFinished(playerId: String)
    }

    var listener: Listener? = null

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private val sessions = ConcurrentHashMap<String, AudioSession>()

    /** Manages the system Equalizer/BassBoost effects per audio session. */
    val equalizer = EqualizerController()

    private val audioManager by lazy { context.getSystemService(Context.AUDIO_SERVICE) as AudioManager }

    private val audioAttrs = AudioAttributes.Builder()
        .setUsage(C.USAGE_MEDIA)
        .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
        .build()

    // Each player gets its own tap so multiple concurrent sessions (crossfade, multi-device)
    // don't cross-contaminate each other's visualizer data.
    private fun buildPlayer(): Pair<ExoPlayer, VisualizerAudioProcessor> {
        val processor = VisualizerAudioProcessor()
        val renderersFactory = object : DefaultRenderersFactory(context) {
            override fun buildAudioSink(
                context: Context,
                enableFloatOutput: Boolean,
                enableAudioTrackPlaybackParams: Boolean,
            ): AudioSink =
                DefaultAudioSink.Builder(context)
                    .setAudioProcessors(arrayOf(processor))
                    .setEnableFloatOutput(enableFloatOutput)
                    .setEnableAudioTrackPlaybackParams(enableAudioTrackPlaybackParams)
                    .build()
        }
        val player = ExoPlayer.Builder(context, renderersFactory)
            .setAudioAttributes(audioAttrs, true)
            .build()
        return player to processor
    }

    /** Assign a known audio session id and attach EQ effects so they're live before playback. */
    private fun setupEq(player: ExoPlayer): Int {
        val sessionId = audioManager.generateAudioSessionId()
        player.setAudioSessionId(sessionId)
        equalizer.attach(sessionId)
        return sessionId
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
                Log.d(TAG, "isPlaying=$isPlaying state=$state volume=${session.player.volume} audioSessionId=${session.player.audioSessionId}")
                listener?.onStateChanged(playerId, state)
            }

            override fun onPlaybackStateChanged(state: Int) {
                val stateName = when (state) {
                    Player.STATE_IDLE -> "IDLE"
                    Player.STATE_BUFFERING -> "BUFFERING"
                    Player.STATE_READY -> "READY"
                    Player.STATE_ENDED -> "ENDED"
                    else -> "UNKNOWN($state)"
                }
                Log.d(TAG, "playbackState=$stateName volume=${session.player.volume}")
                when (state) {
                    Player.STATE_BUFFERING -> listener?.onStateChanged(playerId, "loading")
                    Player.STATE_IDLE -> listener?.onStateChanged(playerId, "stopped")
                    // Use the ExoPlayer callback directly instead of a polling loop — the polling
                    // loop runs on Dispatchers.Main which Android throttles in the background,
                    // causing repeat/next to not fire when the app isn't in the foreground.
                    Player.STATE_ENDED -> scope.launch {
                        if (sessions.remove(playerId) != null) {
                            equalizer.detach(session.audioSessionId)
                            session.player.release()
                            listener?.onPlaybackFinished(playerId)
                        }
                    }
                    else -> {}
                }
            }

            override fun onPlayerError(error: PlaybackException) {
                Log.e(TAG, "ExoPlayer error: ${error.errorCodeName} (${error.errorCode})", error)
                listener?.onStateChanged(playerId, "stopped")
            }

            override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
                val queueIds = session.queueTrackIds ?: return
                val idx = session.player.currentMediaItemIndex
                val newTrackId = queueIds.getOrNull(idx) ?: return
                val previousTrackId = session.currentTrackId
                val wasNaturalCompletion = reason == Player.MEDIA_ITEM_TRANSITION_REASON_AUTO
                session.currentTrackId = newTrackId
                session.currentQueueIndex = idx
                val newGain = session.queueReplayGainFactors?.getOrNull(idx) ?: 1.0f
                session.replayGainFactor = newGain
                session.player.volume = session.baseVolume * newGain
                listener?.onTrackChanged(playerId, newTrackId, idx, previousTrackId, wasNaturalCompletion)
            }
        })

    }

    private fun releaseSession(playerId: String) {
        sessions.remove(playerId)?.let { s ->
            s.fadeJob?.cancel()
            equalizer.detach(s.audioSessionId)
            s.player.stop()
            s.player.release()
        }
    }

    // ── Playback commands ──────────────────────────────────────────────────────

    fun play(streamUrl: String, trackId: String, replayGainDb: Float? = null): String {
        val playerId = UUID.randomUUID().toString()
        val (player, visualizerProcessor) = buildPlayer()
        val sessionId = setupEq(player)
        val gain = gainFactor(replayGainDb)
        player.volume = gain
        val session = AudioSession(player, trackId, 1.0f, gain, audioSessionId = sessionId, visualizerProcessor = visualizerProcessor)
        sessions[playerId] = session
        attachListeners(playerId, session)
        player.setMediaItem(MediaItem.fromUri(streamUrl))
        player.prepare()
        player.playWhenReady = true
        return playerId
    }

    fun resume(playerId: String) {
        val session = sessions[playerId] ?: return
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

    fun setReplayGainEnabled(enabled: Boolean) {
        sessions.values.forEach { session ->
            val factor = if (enabled) session.replayGainFactor else 1.0f
            session.player.volume = session.baseVolume * factor
        }
    }

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

    fun getVisualizerProcessor(playerId: String): VisualizerAudioProcessor? =
        sessions[playerId]?.visualizerProcessor

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
        val (player, visualizerProcessor) = buildPlayer()
        val sessionId = setupEq(player)
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
            audioSessionId = sessionId,
            visualizerProcessor = visualizerProcessor,
        )
        sessions[playerId] = session
        attachListeners(playerId, session)

        Log.d(TAG, "setQueue tracks=${tracks.size} startIdx=$idx volume=$volume initialGain=$initialGain effectiveVol=${volume * initialGain}")
        tracks.forEachIndexed { i, t -> Log.d(TAG, "  track[$i] url=${t.streamUrl.take(80)} replayGain=${t.replayGainDb}") }
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

    // Appends a track to the end of the running queue without disturbing the current item or
    // position. Keeps the session's parallel track-id / gain lists in sync for transitions.
    fun appendToQueue(playerId: String, track: QueueTrack) {
        val session = sessions[playerId] ?: return
        session.player.addMediaItem(MediaItem.fromUri(track.streamUrl))
        session.queueTrackIds = (session.queueTrackIds ?: emptyList()) + track.trackId
        session.queueReplayGainFactors = (session.queueReplayGainFactors ?: emptyList()) + gainFactor(track.replayGainDb)
    }

    fun skipToIndex(playerId: String, index: Int) {
        sessions[playerId]?.player?.seekTo(index, 0L)
    }

    // Reorders the live ExoPlayer playlist and keeps the session's parallel track-id / gain
    // lists in sync so future transitions still resolve the right track/gain at each index.
    fun moveQueueItem(playerId: String, from: Int, to: Int) {
        val session = sessions[playerId] ?: return
        val ids = session.queueTrackIds?.toMutableList() ?: return
        if (from !in ids.indices || to !in ids.indices || from == to) return
        session.player.moveMediaItem(from, to)
        ids.add(to, ids.removeAt(from))
        session.queueTrackIds = ids
        session.queueReplayGainFactors = session.queueReplayGainFactors?.toMutableList()?.also {
            if (from in it.indices) it.add(to, it.removeAt(from))
        }
        // moveMediaItem doesn't fire onMediaItemTransition (the playing item doesn't change),
        // so resync the tracked index from ExoPlayer's own (now-shifted) currentMediaItemIndex.
        session.currentQueueIndex = session.player.currentMediaItemIndex
    }

    // Removes a track from the live queue. If it's the currently playing item, ExoPlayer
    // auto-advances and the existing onMediaItemTransition listener updates current track/index;
    // otherwise we resync currentQueueIndex ourselves since no transition event fires.
    fun removeQueueItem(playerId: String, index: Int) {
        val session = sessions[playerId] ?: return
        val ids = session.queueTrackIds?.toMutableList() ?: return
        if (index !in ids.indices) return
        session.player.removeMediaItem(index)
        ids.removeAt(index)
        session.queueTrackIds = ids
        session.queueReplayGainFactors = session.queueReplayGainFactors?.toMutableList()?.also {
            if (index in it.indices) it.removeAt(index)
        }
        session.currentQueueIndex = session.player.currentMediaItemIndex
        ids.getOrNull(session.currentQueueIndex)?.let { session.currentTrackId = it }
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
        curve: String = "linear",
    ): String {
        val newPlayerId = UUID.randomUUID().toString()
        val (newPlayer, visualizerProcessor) = buildPlayer()
        val sessionId = setupEq(newPlayer)
        val gain = gainFactor(replayGainDb)
        newPlayer.volume = 0f
        val newSession = AudioSession(newPlayer, trackId, targetVolume, gain, audioSessionId = sessionId, visualizerProcessor = visualizerProcessor)
        sessions[newPlayerId] = newSession
        attachListeners(newPlayerId, newSession)
        newPlayer.setMediaItem(MediaItem.fromUri(streamUrl))
        newPlayer.prepare()
        newPlayer.playWhenReady = true

        val steps = 25
        val logarithmic = curve == "logarithmic"
        // Map a 0.0–1.0 ramp position to a volume factor. Logarithmic approximates
        // an equal-power (perceptual) fade; linear keeps the raw position.
        fun curveGain(t: Float): Float = if (logarithmic) 10f.pow((t - 1f) * 2f) else t
        scope.launch {
            val stepMs = (fadeDurationMs / steps).coerceAtLeast(50)
            repeat(steps) { step ->
                delay(stepMs)
                val progress = (step + 1).toFloat() / steps
                sessions[oldPlayerId]?.player?.volume = (targetVolume * curveGain(1f - progress)).coerceAtLeast(0f)
                sessions[newPlayerId]?.player?.volume = targetVolume * curveGain(progress) * gain
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
    var fadeJob: Job? = null,
    var queueTrackIds: List<String>? = null,
    var queueReplayGainFactors: List<Float>? = null,
    var currentQueueIndex: Int = 0,
    val audioSessionId: Int = 0,
    val visualizerProcessor: VisualizerAudioProcessor? = null,
)
