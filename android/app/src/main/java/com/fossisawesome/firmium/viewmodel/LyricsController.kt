package com.fossisawesome.firmium.viewmodel

import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.ApiClient.LyricLine
import com.fossisawesome.firmium.data.model.Song
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.Job

data class LyricsState(
    val lines: List<LyricLine> = emptyList(),
    val synced: Boolean = false,
    val activeLine: Int = -1,
    val isLoading: Boolean = false,
    val isOpen: Boolean = false,
    val trackId: String? = null,
)

// Owns lyrics fetching, caching, and position-sync state for the currently playing track.
class LyricsController(private val scope: CoroutineScope, private val api: ApiClient) {

    private val _state = MutableStateFlow(LyricsState())
    val state: StateFlow<LyricsState> = _state.asStateFlow()

    private var lyricsJob: Job? = null

    fun open() { _state.update { it.copy(isOpen = true) } }
    fun close() { _state.update { it.copy(isOpen = false) } }

    fun fetchForTrack(track: Song) {
        // Skip if we already have lyrics for this track.
        if (_state.value.trackId == track.id && _state.value.lines.isNotEmpty()) return
        lyricsJob?.cancel()
        _state.update { it.copy(isLoading = true, lines = emptyList(), synced = false, activeLine = -1, trackId = track.id) }
        lyricsJob = scope.launch {
            val trackId = track.id
            val result = try {
                api.getLyrics(track.id, track.artist, track.title, track.album, track.duration)
            } catch (e: Exception) { if (e is CancellationException) throw e; null }
            // Guard against a stale fetch racing ahead of a newer track that started before this
            // coroutine was cancelled.
            if (_state.value.trackId != trackId) return@launch
            if (result != null) {
                _state.update { it.copy(isLoading = false, lines = result.lines, synced = result.synced) }
            } else {
                _state.update { it.copy(isLoading = false) }
            }
        }
    }

    // Finds the active lyric line for the given playback position (milliseconds scan).
    // Matches the syncLyricsToPosition logic from playback.js.
    fun syncToPosition(positionSeconds: Double) {
        val ls = _state.value
        if (!ls.synced || ls.lines.isEmpty()) return
        val posMs = (positionSeconds * 1000).toLong()
        var active = -1
        for (i in ls.lines.indices) {
            val startMs = ls.lines[i].startMs ?: break
            if (startMs <= posMs) active = i else break
        }
        if (active != ls.activeLine) {
            _state.update { it.copy(activeLine = active) }
        }
    }

    fun cancel() {
        lyricsJob?.cancel()
    }
}
