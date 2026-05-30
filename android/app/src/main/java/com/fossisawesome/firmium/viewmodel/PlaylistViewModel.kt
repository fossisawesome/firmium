package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.data.model.Song
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

data class PlaylistsUiState(
    val playlists: List<Playlist> = emptyList(),
)

class PlaylistViewModel(app: Application) : AndroidViewModel(app) {

    private val repo = getApplication<FirmiumApplication>().playlists

    val state: StateFlow<PlaylistsUiState> = repo.playlists
        .map { PlaylistsUiState(it) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), PlaylistsUiState())

    fun create(name: String) = viewModelScope.launch { repo.create(name) }

    // Creates a playlist and immediately adds the given songs to it.
    fun createAndAdd(name: String, songs: List<Song>) = viewModelScope.launch {
        val playlist = repo.create(name)
        repo.addTracks(playlist.id, songs)
    }
    fun delete(id: String) = viewModelScope.launch { repo.delete(id) }
    fun rename(id: String, name: String) = viewModelScope.launch { repo.rename(id, name) }
    fun addTracks(id: String, songs: List<Song>) = viewModelScope.launch { repo.addTracks(id, songs) }
    fun removeTrack(playlistId: String, trackId: String) = viewModelScope.launch { repo.removeTrack(playlistId, trackId) }

    fun playlistById(id: String): Playlist? = state.value.playlists.find { it.id == id }
}
