package com.fossisawesome.firmium.audio

import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.WatchAuthManager
import com.fossisawesome.firmium.data.download.WatchDownloadManager
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.data.storage.WatchPreferences
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

// Player state — single source of truth for the watch UI (a future now-playing screen observes
// this, per sub-project 4).
data class PlayerState(
    val queue: List<Song> = emptyList(),
    val queueIndex: Int = -1,
    val playbackState: String = "stopped",
    val currentPosition: Double = 0.0,
    val trackDuration: Double = 0.0,
    val volume: Float = 1.0f,
    val repeatMode: String = "none",
    val shuffleEnabled: Boolean = false,
    val crossfadeEnabled: Boolean = false,
    val crossfadeDurationMs: Int = 3000,
    val crossfadeCurve: String = "linear",
    val gaplessEnabled: Boolean = true,
    val replayGainEnabled: Boolean = true,
    val isSeeking: Boolean = false,
) {
    val currentTrack: Song? get() = queue.getOrNull(queueIndex)
    val hasNext: Boolean get() = queueIndex < queue.size - 1 || repeatMode == "all"
    val hasPrev: Boolean get() = queueIndex > 0
}

// Watch playback orchestration — adapted from the phone's PlaybackController.kt. Drops Android
// Auto entry points, local-library preference, play history, auto-continue radio seeding, and
// ListenBrainz submission (see the watch playback engine spec). Owns queue/transport/scrobble
// logic and drives AudioPlayer + WatchNowPlayingNotifier.
class WatchPlaybackController(
    private val audioPlayer: AudioPlayer,
    private val notifier: WatchNowPlayingNotifier,
    private val api: ApiClient,
    private val auth: WatchAuthManager,
    private val prefs: WatchPreferences,
    private val downloadManager: WatchDownloadManager,
) {

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    private val _state = MutableStateFlow(PlayerState())
    val state: StateFlow<PlayerState> = _state.asStateFlow()

    private var currentPlayerId: String? = null
    private var positionJob: Job? = null

    init {
        scope.launch {
            combine(
                prefs.volume,
                prefs.repeatMode,
                prefs.shuffleEnabled,
                prefs.crossfadeEnabled,
                prefs.crossfadeDuration,
            ) { vol, repeat, shuffle, cf, cfDur ->
                _state.update { it.copy(volume = vol, repeatMode = repeat, shuffleEnabled = shuffle, crossfadeEnabled = cf, crossfadeDurationMs = cfDur) }
            }.collect()
        }
        scope.launch {
            prefs.crossfadeCurve.collect { curve -> _state.update { it.copy(crossfadeCurve = curve) } }
        }
        scope.launch {
            prefs.gaplessEnabled.collect { gap -> _state.update { it.copy(gaplessEnabled = gap) } }
        }
        scope.launch {
            prefs.replayGainEnabled.collect { rg -> _state.update { it.copy(replayGainEnabled = rg) } }
        }

        audioPlayer.listener = object : AudioPlayer.Listener {
            override fun onStateChanged(playerId: String, state: String) {
                if (playerId != currentPlayerId) return
                _state.update { it.copy(playbackState = state) }
                if (state == "playing") startPositionTracking()
                else if (state == "paused" || state == "stopped") stopPositionTracking()
                updateNowPlayingNotification()
            }

            override fun onTrackChanged(playerId: String, trackId: String, index: Int, previousTrackId: String?, wasNaturalCompletion: Boolean) {
                if (playerId != currentPlayerId) return
                // Submit scrobble for the track that just finished playing naturally.
                // onPlaybackFinished only fires at STATE_ENDED (full queue end), so mid-queue
                // natural completions must be handled here.
                if (wasNaturalCompletion && previousTrackId != null) {
                    val finishedSong = _state.value.currentTrack
                    val finishedDuration = finishedSong?.duration ?: _state.value.trackDuration.toInt()
                    scope.launch {
                        api.scrobble(previousTrackId, true)
                        api.reportPlayback(previousTrackId, finishedDuration * 1000L, "stopped")
                    }
                }
                _state.update { it.copy(queueIndex = index) }
                updateNowPlayingNotification()
                scope.launch { scrobbleCurrent(false) }
                scope.launch { reportPlaybackCurrent("starting", 0L) }
            }

            override fun onPlaybackFinished(playerId: String) {
                if (playerId != currentPlayerId) return
                stopPositionTracking()
                scope.launch { scrobbleCurrent(true) }
                scope.launch { reportPlaybackCurrent("stopped", (_state.value.trackDuration * 1000).toLong()) }
                onTrackEnded()
            }
        }
    }

    // ── Queue management ───────────────────────────────────────────────────────

    // Prefers a downloaded local file over streaming; falls back to auth.streamUrl if the
    // track hasn't been downloaded.
    private suspend fun streamUrlFor(song: Song): String {
        val local = downloadManager.localPathFor(song.id)
        return if (local != null) "file://$local" else auth.streamUrl(song.id)
    }

    fun playAt(songs: List<Song>, startIndex: Int) {
        scope.launch {
            try {
                val rgEnabled = _state.value.replayGainEnabled
                val tracks = songs.map { song ->
                    QueueTrack(
                        streamUrl = streamUrlFor(song),
                        trackId = song.id,
                        replayGainDb = if (rgEnabled) (song.replayGainTrack ?: song.replayGainAlbum)?.toFloat() else null,
                    )
                }
                val playerId = audioPlayer.setQueue(tracks, startIndex, _state.value.volume)
                currentPlayerId = playerId
                val actualIndex = startIndex.coerceIn(0, (songs.size - 1).coerceAtLeast(0))
                _state.update { it.copy(
                    queue = songs, queueIndex = actualIndex,
                    playbackState = "loading", currentPosition = 0.0,
                ) }
                updateNowPlayingNotification()
                scrobbleCurrent(false)
                reportPlaybackCurrent("starting", 0L)
            } catch (_: Exception) { /* credentials unavailable or stream setup failed */ }
        }
    }

    fun skipToIndex(index: Int) {
        val pid = currentPlayerId ?: return
        if (index < 0 || index >= _state.value.queue.size) return
        audioPlayer.skipToIndex(pid, index)
        _state.update { it.copy(queueIndex = index, currentPosition = 0.0) }
        updateNowPlayingNotification()
    }

    // Appends a track to the end of the current queue without interrupting playback. If nothing
    // is playing, starts a fresh queue with just this track.
    fun addToQueue(song: Song) {
        val pid = currentPlayerId
        if (pid == null || _state.value.queue.isEmpty()) {
            playAt(listOf(song), 0)
            return
        }
        scope.launch {
            try {
                val rgEnabled = _state.value.replayGainEnabled
                audioPlayer.appendToQueue(pid, QueueTrack(
                    streamUrl = streamUrlFor(song),
                    trackId = song.id,
                    replayGainDb = if (rgEnabled) (song.replayGainTrack ?: song.replayGainAlbum)?.toFloat() else null,
                ))
                _state.update { it.copy(queue = it.queue + song) }
            } catch (_: Exception) { /* stream setup failed */ }
        }
    }

    // ── Transport controls ─────────────────────────────────────────────────────

    fun pause() {
        currentPlayerId?.let { audioPlayer.pause(it) }
        scope.launch { reportPlaybackCurrent("paused") }
    }

    fun resume() {
        currentPlayerId?.let { audioPlayer.resume(it) }
        scope.launch { reportPlaybackCurrent("playing") }
    }

    fun togglePlayPause() {
        when (_state.value.playbackState) {
            "playing" -> pause()
            "paused" -> resume()
            // Queue ended — restart the last track instead of doing nothing.
            "stopped" -> {
                val s = _state.value
                if (s.currentTrack != null) playAt(s.queue, s.queueIndex)
            }
        }
    }

    fun skipToNext() {
        val s = _state.value
        val nextIdx = s.queueIndex + 1
        when {
            s.repeatMode == "one" -> seek(0.0)
            nextIdx < s.queue.size -> {
                if (s.crossfadeEnabled) crossfadeToNext()
                else currentPlayerId?.let { audioPlayer.skipToNext(it) }
            }
            s.repeatMode == "all" -> skipToIndex(0)
        }
    }

    fun skipToPrevious() {
        val s = _state.value
        when {
            s.currentPosition > 3.0 -> seek(0.0)
            s.queueIndex > 0 -> currentPlayerId?.let { audioPlayer.skipToPrevious(it) }
        }
    }

    fun seek(positionSeconds: Double) {
        currentPlayerId?.let { audioPlayer.seek(it, positionSeconds) }
        _state.update { it.copy(currentPosition = positionSeconds) }
    }

    fun setVolume(volume: Float) {
        currentPlayerId?.let { audioPlayer.setVolume(it, volume) }
        _state.update { it.copy(volume = volume) }
        scope.launch { prefs.setVolume(volume) }
    }

    fun setSeekingFlag(seeking: Boolean) { _state.update { it.copy(isSeeking = seeking) } }

    // ── Settings ───────────────────────────────────────────────────────────────

    fun setRepeatMode(mode: String) {
        _state.update { it.copy(repeatMode = mode) }
        scope.launch { prefs.setRepeatMode(mode) }
    }

    fun toggleShuffle() {
        val next = !_state.value.shuffleEnabled
        _state.update { it.copy(shuffleEnabled = next) }
        scope.launch { prefs.setShuffleEnabled(next) }
    }

    fun setCrossfadeEnabled(enabled: Boolean) {
        _state.update { it.copy(crossfadeEnabled = enabled) }
        scope.launch { prefs.setCrossfadeEnabled(enabled) }
    }

    fun setCrossfadeDuration(ms: Int) {
        _state.update { it.copy(crossfadeDurationMs = ms) }
        scope.launch { prefs.setCrossfadeDuration(ms) }
    }

    fun setCrossfadeCurve(curve: String) {
        val normalized = if (curve == "logarithmic") "logarithmic" else "linear"
        _state.update { it.copy(crossfadeCurve = normalized) }
        scope.launch { prefs.setCrossfadeCurve(normalized) }
    }

    fun setGaplessEnabled(enabled: Boolean) {
        _state.update { it.copy(gaplessEnabled = enabled) }
        scope.launch { prefs.setGaplessEnabled(enabled) }
    }

    fun setReplayGainEnabled(enabled: Boolean) {
        _state.update { it.copy(replayGainEnabled = enabled) }
        scope.launch { prefs.setReplayGainEnabled(enabled) }
        audioPlayer.setReplayGainEnabled(enabled)
    }

    // ── Internal helpers ───────────────────────────────────────────────────────

    private fun crossfadeToNext() {
        val s = _state.value
        val pid = currentPlayerId ?: return
        val nextIdx = s.queueIndex + 1
        val nextSong = s.queue.getOrNull(nextIdx) ?: return
        scope.launch {
            scrobbleCurrent(true)
            val newPid = audioPlayer.crossfadeTo(
                oldPlayerId = pid,
                streamUrl = streamUrlFor(nextSong),
                trackId = nextSong.id,
                fadeDurationMs = s.crossfadeDurationMs.toLong(),
                targetVolume = s.volume,
                replayGainDb = if (s.replayGainEnabled) (nextSong.replayGainTrack ?: nextSong.replayGainAlbum)?.toFloat() else null,
                curve = s.crossfadeCurve,
            )
            currentPlayerId = newPid
            _state.update { it.copy(queueIndex = nextIdx, currentPosition = 0.0) }
            updateNowPlayingNotification()
            scrobbleCurrent(false)
            reportPlaybackCurrent("starting", 0L)
        }
    }

    private fun onTrackEnded() {
        val s = _state.value
        when (s.repeatMode) {
            // "one" = repeat once then stop. The ExoPlayer session is already released when
            // onPlaybackFinished fires, so seek+resume would be no-ops — use playAt instead.
            "one" -> {
                setRepeatMode("none")
                if (s.currentTrack != null) playAt(s.queue, s.queueIndex)
            }
            // skipToIndex won't work here — the session was just released. Use playAt to
            // create a fresh ExoPlayer instance with the queue starting from the top.
            "all" -> if (s.queue.isNotEmpty()) playAt(s.queue, 0)
            else -> if (s.hasNext) skipToNext() else {
                _state.update { it.copy(playbackState = "stopped") }
                // Keep the notification alive (paused-style, play icon) so it can restart
                // playback — matches the phone's "keep media controls alive" behavior.
                updateNowPlayingNotification()
            }
        }
    }

    private fun updateNowPlayingNotification() {
        val track = _state.value.currentTrack ?: return
        notifier.update(
            title = track.title,
            artist = track.displayArtist ?: track.artist,
            isPlaying = _state.value.playbackState == "playing",
        )
    }

    private suspend fun scrobbleCurrent(submission: Boolean) {
        val trackId = _state.value.currentTrack?.id ?: return
        api.scrobble(trackId, submission)
    }

    private suspend fun reportPlaybackCurrent(state: String, positionMs: Long? = null) {
        val s = _state.value
        val trackId = s.currentTrack?.id ?: return
        api.reportPlayback(trackId, positionMs ?: (s.currentPosition * 1000).toLong(), state)
    }

    private fun startPositionTracking() {
        positionJob?.cancel()
        positionJob = scope.launch {
            while (isActive) {
                val pid = currentPlayerId ?: break
                val pos = audioPlayer.getPosition(pid)
                val dur = audioPlayer.getDuration(pid) ?: 0.0
                if (!_state.value.isSeeking) {
                    _state.update { it.copy(currentPosition = pos, trackDuration = dur) }
                }
                val s = _state.value
                if (dur > 0 && s.playbackState == "playing" && s.hasNext) {
                    if (s.crossfadeEnabled) {
                        val fadeAt = dur - (s.crossfadeDurationMs / 1000.0)
                        if (pos >= fadeAt) { skipToNext(); break }
                    }
                }
                delay(250)
            }
        }
    }

    private fun stopPositionTracking() {
        positionJob?.cancel()
        positionJob = null
    }
}
