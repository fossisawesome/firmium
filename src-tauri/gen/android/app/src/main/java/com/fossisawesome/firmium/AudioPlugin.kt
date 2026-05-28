package com.fossisawesome.firmium

import android.app.Activity
import android.os.Handler
import android.os.Looper
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlin.math.pow

// ── Arg classes ──────────────────────────────────────────────────────────────

@InvokeArg
class AudioPlayStreamArgs {
    var streamUrl: String = ""
    var trackId: String = ""
    var replayGainDb: Float? = null
}

@InvokeArg
class AudioPlayerIdArgs {
    var playerId: String = ""
}

@InvokeArg
class AudioSeekArgs {
    var playerId: String = ""
    var position: Double = 0.0
}

@InvokeArg
class AudioVolumeArgs {
    var playerId: String = ""
    var volume: Float = 1.0f
}

@InvokeArg
class AudioCrossfadeArgs {
    var oldPlayerId: String = ""
    var streamUrl: String = ""
    var trackId: String = ""
    var fadeDurationMs: Long = 3000
    var targetVolume: Float = 1.0f
    var replayGainDb: Float? = null
}

// ── Session state ─────────────────────────────────────────────────────────────

private data class AudioSession(
    val player: ExoPlayer,
    val trackId: String,
    var baseVolume: Float = 1.0f,
    var replayGainFactor: Float = 1.0f,
    var finishWatchJob: Job? = null,
)

// ── Plugin ────────────────────────────────────────────────────────────────────

// Provides native Android audio playback via ExoPlayer (Media3).
// Mirrors the Rust/rodio command interface so the desktop and mobile audio
// paths share the same JS AudioBridge code.
@TauriPlugin
class AudioPlugin(private val activity: Activity) : Plugin(activity) {

    private val mainHandler = Handler(Looper.getMainLooper())
    private val sessions = ConcurrentHashMap<String, AudioSession>()
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    // Audio attributes for music — requests audio focus so other apps duck.
    private val audioAttrs = AudioAttributes.Builder()
        .setUsage(C.USAGE_MEDIA)
        .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
        .build()

    // ── Internal helpers ──────────────────────────────────────────────────────

    private fun buildPlayer(): ExoPlayer =
        ExoPlayer.Builder(activity)
            .setAudioAttributes(audioAttrs, /* handleAudioFocus= */ true)
            .build()

    private fun gainFactor(gainDb: Float?): Float =
        if (gainDb != null) (10.0.pow(gainDb / 20.0)).toFloat().coerceIn(0.01f, 4.0f)
        else 1.0f

    // Attaches a Player.Listener that emits Tauri plugin events for state changes.
    // Also spawns a coroutine that watches for STATE_ENDED and emits playback-finished.
    private fun attachListeners(playerId: String, session: AudioSession) {
        session.player.addListener(object : Player.Listener {
            override fun onIsPlayingChanged(isPlaying: Boolean) {
                val state = if (isPlaying) "playing"
                    else if (session.player.playbackState == Player.STATE_READY) "paused"
                    else return
                emitState(playerId, state)
            }

            override fun onPlaybackStateChanged(state: Int) {
                when (state) {
                    Player.STATE_BUFFERING -> emitState(playerId, "loading")
                    Player.STATE_READY -> {
                        // onIsPlayingChanged fires separately for playing/paused
                    }
                    Player.STATE_IDLE -> emitState(playerId, "stopped")
                    else -> { /* STATE_ENDED handled by the finish watcher */ }
                }
            }
        })

        // Finish watcher — polls at 100ms so we can clean up and fire the event.
        val job = scope.launch {
            while (sessions.containsKey(playerId)) {
                delay(100)
                val s = sessions[playerId] ?: break
                if (s.player.playbackState == Player.STATE_ENDED) {
                    sessions.remove(playerId)
                    s.player.release()
                    val obj = JSObject()
                    obj.put("playerId", playerId)
                    trigger("playback-finished", obj)
                    break
                }
            }
        }
        session.finishWatchJob = job
    }

    private fun emitState(playerId: String, state: String) {
        val obj = JSObject()
        obj.put("playerId", playerId)
        obj.put("state", state)
        trigger("playback-state-changed", obj)
    }

    // Stops and releases a session by ID, cancelling its finish watcher.
    private fun releaseSession(playerId: String) {
        sessions.remove(playerId)?.let { s ->
            s.finishWatchJob?.cancel()
            s.player.stop()
            s.player.release()
        }
    }

    // ── Commands ──────────────────────────────────────────────────────────────

    // Start streaming a URL immediately. Returns a playerId for subsequent control.
    @Command
    fun playStream(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayStreamArgs::class.java)
        mainHandler.post {
            val playerId = UUID.randomUUID().toString()
            val player = buildPlayer()
            val gainFactor = gainFactor(args.replayGainDb)
            player.volume = gainFactor // initial volume = ReplayGain factor; set by setVolume later
            val session = AudioSession(
                player = player,
                trackId = args.trackId,
                baseVolume = 1.0f,
                replayGainFactor = gainFactor,
            )
            sessions[playerId] = session
            attachListeners(playerId, session)

            player.setMediaItem(MediaItem.fromUri(args.streamUrl))
            player.prepare()
            player.playWhenReady = true

            val result = JSObject()
            result.put("playerId", playerId)
            invoke.resolve(result)
        }
    }

    // Pre-fetch and decode a track in a paused state for gapless playback.
    // Call resumePlayback on the returned playerId to start audio instantly.
    @Command
    fun preloadStream(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayStreamArgs::class.java)
        mainHandler.post {
            val playerId = UUID.randomUUID().toString()
            val player = buildPlayer()
            val gainFactor = gainFactor(args.replayGainDb)
            player.volume = gainFactor
            val session = AudioSession(
                player = player,
                trackId = args.trackId,
                baseVolume = 1.0f,
                replayGainFactor = gainFactor,
            )
            sessions[playerId] = session

            player.setMediaItem(MediaItem.fromUri(args.streamUrl))
            player.prepare()
            player.playWhenReady = false // Preloaded — stays paused until promoted.

            val result = JSObject()
            result.put("playerId", playerId)
            invoke.resolve(result)
        }
    }

    @Command
    fun pausePlayback(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayerIdArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            if (session == null) { invoke.reject("Player not found"); return@post }
            // Brief fade-out to prevent the pause click — 5 steps × 4ms = 20ms.
            scope.launch {
                val vol = session.player.volume
                for (i in 1..5) {
                    session.player.volume = vol * (1f - i / 5f)
                    delay(4)
                }
                session.player.pause()
                session.player.volume = vol // Restore so resume plays at full volume.
                invoke.resolve()
            }
        }
    }

    @Command
    fun resumePlayback(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayerIdArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            if (session == null) { invoke.reject("Player not found"); return@post }
            // Attach listeners if this is a promoted preload (no listeners yet).
            if (session.finishWatchJob == null) {
                attachListeners(args.playerId, session)
            }
            session.player.play()
            invoke.resolve()
        }
    }

    @Command
    fun stopPlayback(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayerIdArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            if (session == null) { invoke.resolve(); return@post }
            // Brief fade-out to eliminate the stop pop.
            scope.launch {
                val vol = session.player.volume
                for (i in 1..5) {
                    session.player.volume = vol * (1f - i / 5f)
                    delay(4)
                }
                releaseSession(args.playerId)
                invoke.resolve()
            }
        }
    }

    @Command
    fun seekPosition(invoke: Invoke) {
        val args = invoke.parseArgs(AudioSeekArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            if (session == null) { invoke.reject("Player not found"); return@post }
            session.player.seekTo((args.position * 1000).toLong())
            invoke.resolve()
        }
    }

    @Command
    fun setVolume(invoke: Invoke) {
        val args = invoke.parseArgs(AudioVolumeArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            if (session == null) { invoke.reject("Player not found"); return@post }
            session.baseVolume = args.volume.coerceIn(0f, 1f)
            session.player.volume = session.baseVolume * session.replayGainFactor
            invoke.resolve()
        }
    }

    @Command
    fun getVolume(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayerIdArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            if (session == null) { invoke.reject("Player not found"); return@post }
            val result = JSObject()
            result.put("volume", session.baseVolume)
            invoke.resolve(result)
        }
    }

    @Command
    fun getPlaybackState(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayerIdArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            val state = when {
                session == null -> "stopped"
                session.player.playbackState == Player.STATE_BUFFERING -> "loading"
                session.player.isPlaying -> "playing"
                session.player.playbackState == Player.STATE_ENDED -> "stopped"
                else -> "paused"
            }
            val result = JSObject()
            result.put("state", state)
            invoke.resolve(result)
        }
    }

    @Command
    fun isPlaybackFinished(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayerIdArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            val finished = session == null || session.player.playbackState == Player.STATE_ENDED
            val result = JSObject()
            result.put("finished", finished)
            invoke.resolve(result)
        }
    }

    @Command
    fun getCurrentPosition(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayerIdArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            val posSeconds = if (session != null) session.player.currentPosition / 1000.0 else 0.0
            val result = JSObject()
            result.put("position", posSeconds)
            invoke.resolve(result)
        }
    }

    @Command
    fun getTrackDuration(invoke: Invoke) {
        val args = invoke.parseArgs(AudioPlayerIdArgs::class.java)
        mainHandler.post {
            val session = sessions[args.playerId]
            val dur = session?.player?.duration
            val result = JSObject()
            if (dur != null && dur != C.TIME_UNSET && dur > 0) {
                result.put("duration", dur / 1000.0)
            } else {
                result.put("duration", null as Any?)
            }
            invoke.resolve(result)
        }
    }

    // Cross-fade from oldPlayerId into a new stream over fadeDurationMs.
    // Starts the new player at volume 0, then ramps old→0 and new→target simultaneously.
    @Command
    fun crossfadeTo(invoke: Invoke) {
        val args = invoke.parseArgs(AudioCrossfadeArgs::class.java)
        mainHandler.post {
            val newPlayerId = UUID.randomUUID().toString()
            val newPlayer = buildPlayer()
            val gainFactor = gainFactor(args.replayGainDb)
            newPlayer.volume = 0f // Start silent — fade task brings it up.
            val newSession = AudioSession(
                player = newPlayer,
                trackId = args.trackId,
                baseVolume = args.targetVolume,
                replayGainFactor = gainFactor,
            )
            sessions[newPlayerId] = newSession
            attachListeners(newPlayerId, newSession)

            newPlayer.setMediaItem(MediaItem.fromUri(args.streamUrl))
            newPlayer.prepare()
            newPlayer.playWhenReady = true

            val oldPlayerId = args.oldPlayerId
            val targetVol = args.targetVolume
            val durationMs = args.fadeDurationMs
            val steps = 25

            scope.launch {
                val stepMs = (durationMs / steps).coerceAtLeast(50)
                for (step in 1..steps) {
                    delay(stepMs)
                    val progress = step.toFloat() / steps
                    sessions[oldPlayerId]?.player?.volume = (targetVol * (1f - progress)).coerceAtLeast(0f)
                    sessions[newPlayerId]?.player?.volume = targetVol * progress * gainFactor
                }
                // Stop old session after fade completes.
                releaseSession(oldPlayerId)
            }

            val result = JSObject()
            result.put("playerId", newPlayerId)
            invoke.resolve(result)
        }
    }
}
