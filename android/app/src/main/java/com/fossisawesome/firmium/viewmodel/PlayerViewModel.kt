package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.audio.AudioPlayer
import com.fossisawesome.firmium.audio.NowPlayingController
import com.fossisawesome.firmium.audio.QueueTrack
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.data.storage.AppPreferences
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

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
    val gaplessEnabled: Boolean = true,
    val isSeeking: Boolean = false,
) {
    val currentTrack: Song? get() = queue.getOrNull(queueIndex)
    val hasNext: Boolean get() = queueIndex < queue.size - 1 || repeatMode == "all"
    val hasPrev: Boolean get() = queueIndex > 0
}

class PlayerViewModel(app: Application) : AndroidViewModel(app) {

    private val audioPlayer: AudioPlayer = getApplication<FirmiumApplication>().audioPlayer
    private val nowPlaying: NowPlayingController = getApplication<FirmiumApplication>().nowPlaying
    private val auth: AuthManager = getApplication<FirmiumApplication>().auth
    private val api: ApiClient = getApplication<FirmiumApplication>().api
    private val prefs: AppPreferences = getApplication<FirmiumApplication>().prefs

    private val _state = MutableStateFlow(PlayerState())
    val state: StateFlow<PlayerState> = _state.asStateFlow()

    private val lyrics = LyricsController(viewModelScope, api)
    val lyricsState: StateFlow<LyricsState> = lyrics.state

    private var currentPlayerId: String? = null
    private var positionJob: Job? = null

    init {
        viewModelScope.launch {
            // Load all persisted settings. combine() supports max 5 args, so chain two.
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
        viewModelScope.launch {
            prefs.gaplessEnabled.collect { gap ->
                _state.update { it.copy(gaplessEnabled = gap) }
            }
        }

        audioPlayer.listener = object : AudioPlayer.Listener {
            override fun onStateChanged(playerId: String, state: String) {
                if (playerId != currentPlayerId) return
                _state.update { it.copy(playbackState = state) }
                if (state == "playing") startPositionTracking()
                else if (state == "paused" || state == "stopped") stopPositionTracking()
                nowPlaying.updatePlaybackState(state == "playing")
            }

            override fun onTrackChanged(playerId: String, trackId: String, index: Int) {
                if (playerId != currentPlayerId) return
                _state.update { it.copy(queueIndex = index) }
                updateNowPlayingNotification()
                viewModelScope.launch { scrobbleCurrent(false) }
                fetchLyricsForCurrent()
            }

            override fun onPlaybackFinished(playerId: String) {
                if (playerId != currentPlayerId) return
                stopPositionTracking()
                viewModelScope.launch { scrobbleCurrent(true) }
                onTrackEnded()
            }
        }

        nowPlaying.listener = object : NowPlayingController.Listener {
            override fun onPlay() {
                // When the queue ended and state is stopped, restart the current track.
                val s = _state.value
                if (s.playbackState == "stopped" && s.currentTrack != null) playAt(s.queue, s.queueIndex)
                else resume()
            }
            override fun onPause() { pause() }
            override fun onNext() { skipToNext() }
            override fun onPrevious() { skipToPrevious() }
            override fun onSeekTo(posMs: Long) { seek(posMs / 1000.0) }
        }
    }

    // ── Queue management ───────────────────────────────────────────────────────

    fun playAt(songs: List<Song>, startIndex: Int) {
        val tracks = songs.map { song ->
            QueueTrack(
                streamUrl = auth.streamUrl(song.id),
                trackId = song.id,
                replayGainDb = (song.replayGainTrack ?: song.replayGainAlbum)?.toFloat(),
            )
        }
        val playerId = audioPlayer.setQueue(tracks, startIndex, _state.value.volume)
        currentPlayerId = playerId
        val actualIndex = startIndex.coerceIn(0, (songs.size - 1).coerceAtLeast(0))
        _state.update { it.copy(queue = songs, queueIndex = actualIndex, playbackState = "loading", currentPosition = 0.0) }
        updateNowPlayingNotification()
        viewModelScope.launch { scrobbleCurrent(false) }
        fetchLyricsForCurrent()
    }

    fun skipToIndex(index: Int) {
        val pid = currentPlayerId ?: return
        if (index < 0 || index >= _state.value.queue.size) return
        audioPlayer.skipToIndex(pid, index)
        _state.update { it.copy(queueIndex = index, currentPosition = 0.0) }
        updateNowPlayingNotification()
    }

    // ── Transport controls ─────────────────────────────────────────────────────

    fun pause() { currentPlayerId?.let { audioPlayer.pause(it) } }
    fun resume() { currentPlayerId?.let { audioPlayer.resume(it) } }

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
        when {
            s.repeatMode == "one" -> seek(0.0)
            s.queueIndex < s.queue.size - 1 -> {
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
        viewModelScope.launch { prefs.setVolume(volume) }
    }

    fun setSeekingFlag(seeking: Boolean) { _state.update { it.copy(isSeeking = seeking) } }

    // ── Settings ───────────────────────────────────────────────────────────────

    fun setRepeatMode(mode: String) {
        _state.update { it.copy(repeatMode = mode) }
        viewModelScope.launch { prefs.setRepeatMode(mode) }
    }

    fun toggleShuffle() {
        val next = !_state.value.shuffleEnabled
        _state.update { it.copy(shuffleEnabled = next) }
        viewModelScope.launch { prefs.setShuffleEnabled(next) }
    }

    fun setCrossfadeEnabled(enabled: Boolean) {
        _state.update { it.copy(crossfadeEnabled = enabled) }
        viewModelScope.launch { prefs.setCrossfadeEnabled(enabled) }
    }

    fun setCrossfadeDuration(ms: Int) {
        _state.update { it.copy(crossfadeDurationMs = ms) }
        viewModelScope.launch { prefs.setCrossfadeDuration(ms) }
    }

    fun setGaplessEnabled(enabled: Boolean) {
        _state.update { it.copy(gaplessEnabled = enabled) }
        viewModelScope.launch { prefs.setGaplessEnabled(enabled) }
    }

    // ── Lyrics ─────────────────────────────────────────────────────────────────

    fun openLyrics() { lyrics.open() }
    fun closeLyrics() { lyrics.close() }

    // ── Internal helpers ───────────────────────────────────────────────────────

    private fun fetchLyricsForCurrent() {
        _state.value.currentTrack?.let { lyrics.fetchForTrack(it) }
    }

    private fun crossfadeToNext() {
        val s = _state.value
        val pid = currentPlayerId ?: return
        val nextIdx = s.queueIndex + 1
        val nextSong = s.queue.getOrNull(nextIdx) ?: return
        viewModelScope.launch { scrobbleCurrent(true) }
        val newPid = audioPlayer.crossfadeTo(
            oldPlayerId = pid,
            streamUrl = auth.streamUrl(nextSong.id),
            trackId = nextSong.id,
            fadeDurationMs = s.crossfadeDurationMs.toLong(),
            targetVolume = s.volume,
            replayGainDb = (nextSong.replayGainTrack ?: nextSong.replayGainAlbum)?.toFloat(),
        )
        currentPlayerId = newPid
        _state.update { it.copy(queueIndex = nextIdx, currentPosition = 0.0) }
        updateNowPlayingNotification()
        viewModelScope.launch { scrobbleCurrent(false) }
        fetchLyricsForCurrent()
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
            "all" -> if (s.queue.isNotEmpty()) skipToIndex(0)
            else -> if (s.hasNext) skipToNext() else {
                _state.update { it.copy(playbackState = "stopped") }
                // Keep the media session and notification alive so OS media controls
                // (lock screen, headset buttons) still function and can restart playback.
                nowPlaying.updatePlaybackState(false)
            }
        }
    }

    private fun updateNowPlayingNotification() {
        val track = _state.value.currentTrack ?: return
        nowPlaying.update(
            title = track.title,
            artist = track.displayArtist ?: track.artist,
            album = track.album,
            coverUrl = track.coverArt?.let { auth.coverArtUrl(it, 512) },
            isPlaying = _state.value.playbackState == "playing",
        )
    }

    private suspend fun scrobbleCurrent(submission: Boolean) {
        val trackId = _state.value.currentTrack?.id ?: return
        api.scrobble(trackId, submission)
    }

    private fun startPositionTracking() {
        positionJob?.cancel()
        positionJob = viewModelScope.launch {
            while (isActive) {
                val pid = currentPlayerId ?: break
                val pos = audioPlayer.getPosition(pid)
                val dur = audioPlayer.getDuration(pid) ?: 0.0
                if (!_state.value.isSeeking) {
                    _state.update { it.copy(currentPosition = pos, trackDuration = dur) }
                }
                if (lyricsState.value.isOpen) lyrics.syncToPosition(pos)
                // Push position to notification so the lock-screen seekbar stays live.
                nowPlaying.updatePosition((pos * 1000).toLong(), (dur * 1000).toLong(), _state.value.playbackState == "playing")
                val s = _state.value
                if (dur > 0 && s.crossfadeEnabled && s.playbackState == "playing") {
                    val fadeAt = dur - (s.crossfadeDurationMs / 1000.0)
                    if (pos >= fadeAt && s.hasNext) { skipToNext(); break }
                }
                delay(250)
            }
        }
    }

    private fun stopPositionTracking() {
        positionJob?.cancel()
        positionJob = null
    }

    override fun onCleared() {
        super.onCleared()
        stopPositionTracking()
        lyrics.cancel()
    }
}
