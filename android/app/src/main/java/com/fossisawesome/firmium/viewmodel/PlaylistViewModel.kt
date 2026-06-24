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
        } catch (e: Exception) {
            android.util.Log.d("PlaylistVM", "server playlist refresh failed, ignoring", e)
            /* keep showing previously fetched server playlists */
        }
    }

    // Loads (and caches) a server-only playlist's tracks for the detail screen.
    fun loadServerPlaylistTracks(serverId: String) = viewModelScope.launch {
        try {
            val tracks = api.getPlaylistTracks(serverId)
            serverTracksCache.update { it + (serverId to tracks) }
        } catch (e: Exception) {
            android.util.Log.d("PlaylistVM", "server playlist track load failed, ignoring", e)
            /* leave cache empty; detail screen shows no tracks */
        }
    }

    fun serverTracksFor(serverId: String): ServerPlaylistTracks? = serverTracksCache.value[serverId]

    fun create(name: String) = viewModelScope.launch { repo.create(name) }

    // Creates a playlist and immediately adds the given songs to it.
    fun createAndAdd(name: String, songs: List<Song>) = viewModelScope.launch {
        val playlist = repo.create(name)
        repo.addTracks(playlist.id, songs)
    }
    fun syncNow(id: String) = viewModelScope.launch { repo.syncNow(id) }
    fun delete(id: String) = viewModelScope.launch { repo.delete(id) }
    fun rename(id: String, name: String) = viewModelScope.launch { repo.rename(id, name) }
    fun addTracks(id: String, songs: List<Song>) = viewModelScope.launch { repo.addTracks(id, songs) }

    // Adds tracks directly to a server-only playlist (no local entry to update).
    fun addTracksToServerOnly(serverId: String, songs: List<Song>) = viewModelScope.launch {
        try {
            api.updatePlaylist(serverId, songIdsToAdd = songs.map { it.id })
        } catch (e: Exception) {
            android.util.Log.d("PlaylistVM", "server-only addTracks failed, ignoring", e)
            /* best-effort, same as local sync */
        }
    }

    // Adds tracks to whichever playlist `item` represents, local/synced or server-only.
    fun addTracksTo(item: PlaylistListItem, songs: List<Song>) = when (item) {
        is PlaylistListItem.Local -> addTracks(item.playlist.id, songs)
        is PlaylistListItem.ServerOnly -> addTracksToServerOnly(item.server.id, songs)
    }
    fun removeTrack(playlistId: String, trackId: String) = viewModelScope.launch { repo.removeTrack(playlistId, trackId) }

    // Removes a track from a server-only playlist by index, updating the cached
    // track list and the server.
    fun removeServerTrack(serverId: String, index: Int) = viewModelScope.launch {
        val current = serverTracksCache.value[serverId] ?: return@launch
        if (index < 0 || index >= current.tracks.size) return@launch
        val tracks = current.tracks.toMutableList().also { it.removeAt(index) }
        serverTracksCache.update { it + (serverId to current.copy(tracks = tracks)) }
        try {
            api.updatePlaylist(serverId, songIndicesToRemove = listOf(index))
        } catch (e: Exception) {
            android.util.Log.d("PlaylistVM", "server-only removeTrack failed, ignoring", e)
            /* best-effort */
        }
    }

    fun moveTrack(playlistId: String, from: Int, to: Int) = viewModelScope.launch { repo.moveTrack(playlistId, from, to) }

    // Reorders a server-only playlist's cached tracks and pushes the new order to the
    // server (remove all original indices, re-add song IDs in the new order).
    fun moveServerTrack(serverId: String, from: Int, to: Int) = viewModelScope.launch {
        val current = serverTracksCache.value[serverId] ?: return@launch
        if (from < 0 || from >= current.tracks.size || to < 0 || to >= current.tracks.size || from == to) return@launch
        val tracks = current.tracks.toMutableList()
        val moved = tracks.removeAt(from)
        tracks.add(to, moved)
        serverTracksCache.update { it + (serverId to current.copy(tracks = tracks)) }
        try {
            api.updatePlaylist(serverId, songIndicesToRemove = tracks.indices.toList(), songIdsToAdd = tracks.map { it.id })
        } catch (e: Exception) {
            android.util.Log.d("PlaylistVM", "server-only moveTrack failed, ignoring", e)
            /* best-effort */
        }
    }

    fun playlistById(id: String): Playlist? = state.value.playlists.find { it.id == id }
}
