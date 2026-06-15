package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.data.model.ServerPlaylist
import com.fossisawesome.firmium.data.model.ServerPlaylistTracks
import com.fossisawesome.firmium.data.model.Song
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

sealed class PlaylistListItem {
    abstract val id: String
    abstract val name: String
    abstract val trackCount: Int
    abstract val isSynced: Boolean  // controls the cloud badge

    data class Local(val playlist: Playlist) : PlaylistListItem() {
        override val id get() = playlist.id
        override val name get() = playlist.name
        override val trackCount get() = playlist.tracks.size
        override val isSynced get() = playlist.serverId != null
    }

    data class ServerOnly(val server: ServerPlaylist) : PlaylistListItem() {
        override val id get() = "server-${server.id}"
        override val name get() = server.name
        override val trackCount get() = server.songCount
        override val isSynced get() = true
    }
}

// Merges local playlists with the server's playlist list into one display list,
// matching local entries with `serverId` to their server counterpart.
fun mergePlaylists(local: List<Playlist>, server: List<ServerPlaylist>): List<PlaylistListItem> {
    val matchedIds = local.mapNotNull { it.serverId }.toSet()
    val localItems = local.map { PlaylistListItem.Local(it) }
    val serverOnly = server.filter { it.id !in matchedIds }.map { PlaylistListItem.ServerOnly(it) }
    return localItems + serverOnly
}

data class PlaylistsUiState(
    val playlists: List<Playlist> = emptyList(),
    val serverPlaylists: List<ServerPlaylist> = emptyList(),
    val items: List<PlaylistListItem> = emptyList(),
)

class PlaylistViewModel(app: Application) : AndroidViewModel(app) {

    private val repo = getApplication<FirmiumApplication>().playlists
    private val api = getApplication<FirmiumApplication>().api

    private val _serverPlaylists = MutableStateFlow<List<ServerPlaylist>>(emptyList())
    private val serverTracksCache = MutableStateFlow<Map<String, ServerPlaylistTracks>>(emptyMap())
    val serverTracks: StateFlow<Map<String, ServerPlaylistTracks>> = serverTracksCache

    val state: StateFlow<PlaylistsUiState> = combine(repo.playlists, _serverPlaylists) { local, server ->
        PlaylistsUiState(local, server, mergePlaylists(local, server))
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), PlaylistsUiState())

    // Fetches the server's playlist list and retries any local playlists that
    // haven't been synced yet (e.g. created while offline).
    fun refreshServerPlaylists() = viewModelScope.launch {
        try {
            val fetched = api.getPlaylists()
            _serverPlaylists.value = fetched
            repo.retryPendingCreates(fetched)
        } catch (_: Exception) { /* keep showing previously fetched server playlists */ }
    }

    // Loads (and caches) a server-only playlist's tracks for the detail screen.
    fun loadServerPlaylistTracks(serverId: String) = viewModelScope.launch {
        try {
            val tracks = api.getPlaylistTracks(serverId)
            serverTracksCache.update { it + (serverId to tracks) }
        } catch (_: Exception) { /* leave cache empty; detail screen shows no tracks */ }
    }

    fun serverTracksFor(serverId: String): ServerPlaylistTracks? = serverTracksCache.value[serverId]

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
