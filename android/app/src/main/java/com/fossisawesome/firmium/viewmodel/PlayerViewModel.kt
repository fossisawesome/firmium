package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.audio.PlaybackController
import com.fossisawesome.firmium.audio.PlayerState
import com.fossisawesome.firmium.data.RadioSeeder
import com.fossisawesome.firmium.data.toUserError
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.model.Song
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

data class SimilarTracksState(
    val isLoading: Boolean = false,
    val matches: List<ApiClient.SimilarMatch> = emptyList(),
    val error: String? = null,
)

// Thin Activity-scoped facade over the app-scoped PlaybackController. Playback state and transport
// live in the controller (shared with Android Auto); this ViewModel only adds the UI-only concerns
// — lyrics and similar-tracks — and exposes the controller's state to Compose.
class PlayerViewModel(app: Application) : AndroidViewModel(app) {

    private val controller: PlaybackController = getApplication<FirmiumApplication>().playback
    private val api: ApiClient = getApplication<FirmiumApplication>().api
    private val radioSeeder = RadioSeeder(api)

    val state: StateFlow<PlayerState> get() = controller.state

    private val lyrics = LyricsController(viewModelScope, api)
    val lyricsState: StateFlow<LyricsState> = lyrics.state

    private val _similarTracksState = kotlinx.coroutines.flow.MutableStateFlow(SimilarTracksState())
    val similarTracksState: StateFlow<SimilarTracksState> = _similarTracksState

    init {
        // Drive lyrics from the controller's state: fetch on track change, sync while the sheet is open.
        viewModelScope.launch {
            var lastTrackId: String? = null
            controller.state.collect { s ->
                val track = s.currentTrack
                if (track != null && track.id != lastTrackId) {
                    lastTrackId = track.id
                    lyrics.fetchForTrack(track)
                }
                if (lyricsState.value.isOpen) lyrics.syncToPosition(s.currentPosition)
            }
        }
    }

    // ── Transport (delegate to controller) ───────────────────────────────────────

    fun playAt(songs: List<Song>, startIndex: Int) = controller.playAt(songs, startIndex)

    // Start Radio: seed from an item and play immediately.
    fun startRadio(seed: Song) {
        viewModelScope.launch {
            val seeded = radioSeeder.seedFrom(seed)
            val tracks = listOf(seed) + seeded
            if (tracks.isNotEmpty()) controller.playAt(tracks, 0)
        }
    }

    // Mood Mix: build a shuffled energy-band queue and play it. onResult reports the
    // number of tracks so the UI can surface an empty-result message.
    fun playMoodMix(energy: RadioSeeder.Energy, genre: String?, onResult: (Int) -> Unit = {}) {
        viewModelScope.launch {
            try {
                val tracks = radioSeeder.buildMoodMix(energy, genre)
                if (tracks.isNotEmpty()) controller.playAt(tracks, 0)
                onResult(tracks.size)
            } catch (e: Exception) {
                if (e is kotlinx.coroutines.CancellationException) throw e
                if (e !is com.fossisawesome.firmium.data.api.SessionExpiredException) {
                    getApplication<FirmiumApplication>().errors.report(e.toUserError())
                }
            }
        }
    }
    fun skipToIndex(index: Int) = controller.skipToIndex(index)
    fun addToQueue(song: Song) = controller.addToQueue(song)
    fun addNextInQueue(song: Song) = controller.addNextInQueue(song)
    fun moveQueueItem(from: Int, to: Int) = controller.moveQueueItem(from, to)
    fun removeQueueItem(index: Int) = controller.removeQueueItem(index)
    fun pause() = controller.pause()
    fun resume() = controller.resume()
    fun togglePlayPause() = controller.togglePlayPause()
    fun skipToNext() = controller.skipToNext()
    fun skipToPrevious() = controller.skipToPrevious()
    fun seek(positionSeconds: Double) = controller.seek(positionSeconds)
    fun setVolume(volume: Float) = controller.setVolume(volume)
    fun setSeekingFlag(seeking: Boolean) = controller.setSeekingFlag(seeking)

    // ── Settings (delegate to controller) ────────────────────────────────────────

    fun setRepeatMode(mode: String) = controller.setRepeatMode(mode)
    fun toggleShuffle() = controller.toggleShuffle()
    fun setCrossfadeEnabled(enabled: Boolean) = controller.setCrossfadeEnabled(enabled)
    fun setCrossfadeDuration(ms: Int) = controller.setCrossfadeDuration(ms)
    fun setCrossfadeCurve(curve: String) = controller.setCrossfadeCurve(curve)
    fun setGaplessEnabled(enabled: Boolean) = controller.setGaplessEnabled(enabled)
    fun setReplayGainEnabled(enabled: Boolean) = controller.setReplayGainEnabled(enabled)
    fun setVisualizerEnabled(enabled: Boolean) = controller.setVisualizerEnabled(enabled)
    fun setVisualizerType(type: String) = controller.setVisualizerType(type)

    // ── Rating ─────────────────────────────────────────────────────────────────

    fun setRating(songId: String, rating: Int) {
        viewModelScope.launch {
            api.setRating(songId, rating)
            controller.updateTrackRating(songId, rating)
        }
    }

    // ── Favorites ──────────────────────────────────────────────────────────────

    fun toggleCurrentTrackStar() {
        val track = state.value.currentTrack ?: return
        val newStarred = !track.starred
        viewModelScope.launch {
            if (newStarred) api.star(songId = track.id) else api.unstar(songId = track.id)
            controller.updateTrackStarred(track.id, newStarred)
        }
    }

    // ── Lyrics ─────────────────────────────────────────────────────────────────

    fun openLyrics() { lyrics.open() }
    fun closeLyrics() { lyrics.close() }

    // ── Similar tracks ─────────────────────────────────────────────────────────

    fun hasSonicSimilarity(): Boolean = api.hasExtension("sonicSimilarity")

    fun fetchSimilarTracks() {
        val track = state.value.currentTrack ?: return
        _similarTracksState.value = SimilarTracksState(isLoading = true)
        viewModelScope.launch {
            _similarTracksState.value = try {
                val matches = if (hasSonicSimilarity()) {
                    api.getSonicSimilarTracks(track.id)
                } else {
                    api.getSimilarTracksFallback(track.id, track.artistId, track.genres.firstOrNull())
                }
                if (matches.isEmpty()) SimilarTracksState(error = "No similar tracks found")
                else SimilarTracksState(matches = matches)
            } catch (e: Exception) {
                SimilarTracksState(error = "No similar tracks found")
            }
        }
    }

    fun clearSimilarTracks() {
        _similarTracksState.value = SimilarTracksState()
    }

    override fun onCleared() {
        super.onCleared()
        lyrics.cancel()
    }
}
