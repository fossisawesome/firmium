package com.fossisawesome.firmium.audio

import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.local.LocalLibraryRepository
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.data.storage.AppPreferences
import com.fossisawesome.firmium.data.storage.PlaylistRepository
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

// Player state — single source of truth for both the phone UI (PlayerViewModel observes this)
// and Android Auto (FirmiumMediaBrowserService drives playback through the controller).
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
    val replayGainEnabled: Boolean = true,
    val isSeeking: Boolean = false,
    val audioSessionId: Int = 0,
) {
    val currentTrack: Song? get() = queue.getOrNull(queueIndex)
    val hasNext: Boolean get() = queueIndex < queue.size - 1 || repeatMode == "all"
    val hasPrev: Boolean get() = queueIndex > 0
}

// Application-scoped playback orchestration. Hoisted out of PlayerViewModel so Android Auto can
// browse and control playback while no Activity (and therefore no ViewModel) exists. Owns the
// queue/transport/scrobble/position logic and drives AudioPlayer + NowPlayingController. The phone
// UI (PlayerViewModel) delegates to this and exposes `state`; lyrics/similar-tracks remain UI-side.
class PlaybackController(
    private val audioPlayer: AudioPlayer,
    private val nowPlaying: NowPlayingController,
    private val api: ApiClient,
    private val auth: AuthManager,
    private val localLibrary: LocalLibraryRepository,
    private val prefs: AppPreferences,
    private val playlists: PlaylistRepository,
) {

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    private val _state = MutableStateFlow(PlayerState())
    val state: StateFlow<PlayerState> = _state.asStateFlow()

    private var currentPlayerId: String? = null
    private var positionJob: Job? = null

    init {
        scope.launch {
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
                nowPlaying.updatePlaybackState(state == "playing")
            }

            override fun onTrackChanged(playerId: String, trackId: String, index: Int) {
                if (playerId != currentPlayerId) return
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
            override fun onPlayFromMediaId(mediaId: String) { playFromMediaId(mediaId) }
            override fun onPlayFromSearch(query: String) { playFromSearch(query) }
        }
    }

    // ── Queue management ───────────────────────────────────────────────────────

    // Local library tracks are played directly from MediaStore via their content:// URI.
    // For server tracks, prefer a locally-downloaded copy if one exists — avoids unnecessary
    // streaming and lets already-downloaded tracks play offline.
    private suspend fun streamUrlFor(song: Song): String {
        if (song.id.startsWith("local:")) {
            return localLibrary.getTrackUri(song.id)?.toString() ?: ""
        }
        val local = localLibrary.findLocalMatch(song.title, song.artist, song.album)
        if (local != null) {
            val uri = localLibrary.getTrackUri(local.id)
            if (uri != null) return uri.toString()
        }
        return auth.streamUrl(song.id)
    }

    fun playAt(songs: List<Song>, startIndex: Int) {
        scope.launch {
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
                audioSessionId = audioPlayer.getAudioSessionId(playerId),
            ) }
            updateNowPlayingNotification()
            scrobbleCurrent(false)
            reportPlaybackCurrent("starting", 0L)
        }
    }

    fun skipToIndex(index: Int) {
        val pid = currentPlayerId ?: return
        if (index < 0 || index >= _state.value.queue.size) return
        audioPlayer.skipToIndex(pid, index)
        _state.update { it.copy(queueIndex = index, currentPosition = 0.0) }
        updateNowPlayingNotification()
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

    fun setGaplessEnabled(enabled: Boolean) {
        _state.update { it.copy(gaplessEnabled = enabled) }
        scope.launch { prefs.setGaplessEnabled(enabled) }
    }

    fun setReplayGainEnabled(enabled: Boolean) {
        _state.update { it.copy(replayGainEnabled = enabled) }
        scope.launch { prefs.setReplayGainEnabled(enabled) }
        audioPlayer.setReplayGainEnabled(enabled)
    }

    // ── Android Auto entry points ────────────────────────────────────────────────

    // Resolves a browse-tree media id to a queue and starts playback. Called from the media
    // session's onPlayFromMediaId (Android Auto tap on a track/album/playlist).
    fun playFromMediaId(mediaId: String) {
        scope.launch {
            when (val node = MediaTree.parse(mediaId)) {
                is MediaNode.AlbumTrack -> {
                    val tracks = albumTracks(node.albumId)
                    if (tracks.isNotEmpty()) playAt(tracks, tracks.indexOfFirst { it.id == node.songId }.coerceAtLeast(0))
                }
                is MediaNode.PlaylistTrack -> {
                    val tracks = playlistTracks(node.playlistId)
                    if (tracks.isNotEmpty()) playAt(tracks, tracks.indexOfFirst { it.id == node.songId }.coerceAtLeast(0))
                }
                is MediaNode.Album -> { val t = albumTracks(node.albumId); if (t.isNotEmpty()) playAt(t, 0) }
                is MediaNode.Playlist -> { val t = playlistTracks(node.playlistId); if (t.isNotEmpty()) playAt(t, 0) }
                else -> { /* category nodes are not playable */ }
            }
        }
    }

    // Voice / search playback ("Hey Google, play X on Firmium"). A blank query means "just play
    // music" — resume the current queue, otherwise start the most recent album.
    fun playFromSearch(query: String) {
        scope.launch {
            if (query.isBlank()) {
                val s = _state.value
                if (s.queue.isNotEmpty()) { if (s.playbackState != "playing") resume(); return@launch }
                val album = (if (auth.isAuthenticated) api.getRecentAlbums(1) else localLibrary.getRecentAlbums(1)).firstOrNull() ?: return@launch
                val tracks = albumTracks(album.id)
                if (tracks.isNotEmpty()) playAt(tracks, 0)
                return@launch
            }
            val results = if (auth.isAuthenticated) api.search(query) else localLibrary.search(query)
            when {
                results.songs.isNotEmpty() -> playAt(results.songs, 0)
                results.albums.isNotEmpty() -> { val t = albumTracks(results.albums.first().id); if (t.isNotEmpty()) playAt(t, 0) }
            }
        }
    }

    private suspend fun albumTracks(albumId: String): List<Song> =
        try {
            if (auth.isAuthenticated) api.getAlbumDetail(albumId).tracks
            else localLibrary.getAlbumDetail(albumId).tracks
        } catch (_: Exception) { emptyList() }

    // Local repository playlists carry their tracks inline; everything else is a server playlist id.
    private suspend fun playlistTracks(playlistId: String): List<Song> {
        playlists.playlists.first().find { it.id == playlistId }?.let { return it.tracks }
        return try { api.getPlaylistTracks(playlistId).tracks } catch (_: Exception) { emptyList() }
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
            )
            currentPlayerId = newPid
            _state.update { it.copy(
                queueIndex = nextIdx, currentPosition = 0.0,
                audioSessionId = audioPlayer.getAudioSessionId(newPid),
            ) }
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
        val coverArt = track.coverArt
        nowPlaying.update(
            title = track.title,
            artist = track.displayArtist ?: track.artist,
            album = track.album,
            coverUrl = coverArt?.let { if (it.startsWith("file://")) it else auth.coverArtUrl(it, 512) },
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
                // Push position to notification so the lock-screen seekbar stays live.
                nowPlaying.updatePosition((pos * 1000).toLong(), (dur * 1000).toLong(), _state.value.playbackState == "playing")
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
