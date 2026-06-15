package com.fossisawesome.firmium.data.storage

import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.data.model.ServerPlaylist
import com.fossisawesome.firmium.data.model.Song
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import java.util.UUID

// Persists playlists as a JSON array in DataStore.
// Mirrors the playlists custom store from stores.js (localStorage-backed).
// Create/rename/delete/track changes are also pushed to the server on a best-effort
// basis (errors are swallowed) — mirrors desktop's Api.createPlaylist/updatePlaylist sync.
class PlaylistRepository(private val prefs: AppPreferences, private val api: ApiClient) {

    private val gson = Gson()
    private val listType = object : TypeToken<List<Playlist>>() {}.type

    val playlists: Flow<List<Playlist>> = prefs.playlistsJson.map { json ->
        if (json.isNullOrBlank()) emptyList()
        else runCatching { gson.fromJson<List<Playlist>>(json, listType) }.getOrDefault(emptyList())
    }

    private suspend fun load(): MutableList<Playlist> {
        val json = prefs.playlistsJson.first()
        return if (json.isNullOrBlank()) mutableListOf()
        else runCatching { gson.fromJson<MutableList<Playlist>>(json, listType) }.getOrDefault(mutableListOf())
    }

    private suspend fun save(list: List<Playlist>) =
        prefs.setPlaylistsJson(gson.toJson(list))

    private suspend fun mutate(block: MutableList<Playlist>.() -> Unit) {
        val list = load()
        list.block()
        save(list)
    }

    suspend fun create(name: String): Playlist {
        val p = Playlist(id = UUID.randomUUID().toString(), name = name)
        mutate { add(0, p) }
        syncCreate(p)
        return p
    }

    // Attempts to create this playlist on the server, recording the result (serverId
    // on success, or an incremented attempt count on failure).
    private suspend fun syncCreate(p: Playlist) {
        try {
            val serverPl = api.createPlaylist(p.name)
            if (p.tracks.isNotEmpty()) {
                api.updatePlaylist(serverPl.id, songIdsToAdd = p.tracks.map { it.id })
            }
            mutate {
                val idx = indexOfFirst { it.id == p.id }
                if (idx >= 0) set(idx, get(idx).copy(serverId = serverPl.id, createPending = false))
            }
        } catch (e: Exception) {
            mutate {
                val idx = indexOfFirst { it.id == p.id }
                if (idx >= 0) {
                    val attempts = get(idx).createAttempts + 1
                    set(idx, get(idx).copy(createAttempts = attempts, createPending = attempts < 3))
                }
            }
        }
    }

    // Manually triggers a sync for a single local-only playlist (e.g. user tapped "Sync").
    suspend fun syncNow(id: String) {
        val p = load().find { it.id == id } ?: return
        if (p.serverId != null) return
        syncCreate(p)
    }

    suspend fun delete(id: String) {
        val serverId = load().find { it.id == id }?.serverId
        mutate { removeIf { it.id == id } }
        if (serverId != null) {
            try { api.deletePlaylist(serverId) } catch (_: Exception) { /* best-effort */ }
        }
    }

    suspend fun rename(id: String, name: String) {
        var serverId: String? = null
        mutate {
            val idx = indexOfFirst { it.id == id }
            if (idx >= 0) {
                serverId = get(idx).serverId
                set(idx, get(idx).copy(name = name))
            }
        }
        if (serverId != null) {
            try { api.updatePlaylist(serverId!!, name = name) } catch (_: Exception) { /* best-effort */ }
        }
    }

    // Appends tracks, skipping duplicates by song id.
    suspend fun addTracks(id: String, songs: List<Song>) {
        var serverId: String? = null
        var newSongs: List<Song> = emptyList()
        mutate {
            val idx = indexOfFirst { it.id == id }
            if (idx < 0) return@mutate
            val existing = get(idx)
            val existingIds = existing.tracks.map { it.id }.toSet()
            newSongs = songs.filter { it.id !in existingIds }
            serverId = existing.serverId
            set(idx, existing.copy(tracks = existing.tracks + newSongs))
        }
        if (serverId != null && newSongs.isNotEmpty()) {
            try { api.updatePlaylist(serverId!!, songIdsToAdd = newSongs.map { it.id }) } catch (_: Exception) { /* best-effort */ }
        }
    }

    // Moves a track within the playlist and, if synced, pushes the new order to the
    // server by removing every original index and re-adding song IDs in the new order
    // (OpenSubsonic's updatePlaylist has no native "move" operation).
    suspend fun moveTrack(id: String, from: Int, to: Int) {
        var serverId: String? = null
        var newTracks: List<Song>? = null
        mutate {
            val idx = indexOfFirst { it.id == id }
            if (idx < 0) return@mutate
            val existing = get(idx)
            if (from < 0 || from >= existing.tracks.size || to < 0 || to >= existing.tracks.size || from == to) return@mutate
            val tracks = existing.tracks.toMutableList()
            val moved = tracks.removeAt(from)
            tracks.add(to, moved)
            serverId = existing.serverId
            newTracks = tracks
            set(idx, existing.copy(tracks = tracks))
        }
        if (serverId != null && newTracks != null) {
            try {
                api.updatePlaylist(
                    serverId!!,
                    songIndicesToRemove = newTracks!!.indices.toList(),
                    songIdsToAdd = newTracks!!.map { it.id },
                )
            } catch (_: Exception) { /* best-effort */ }
        }
    }

    suspend fun removeTrack(id: String, trackId: String) {
        var serverId: String? = null
        var removedIndex = -1
        mutate {
            val idx = indexOfFirst { it.id == id }
            if (idx < 0) return@mutate
            val existing = get(idx)
            removedIndex = existing.tracks.indexOfFirst { it.id == trackId }
            serverId = existing.serverId
            set(idx, existing.copy(tracks = existing.tracks.filter { it.id != trackId }))
        }
        if (serverId != null && removedIndex >= 0) {
            try { api.updatePlaylist(serverId!!, songIndicesToRemove = listOf(removedIndex)) } catch (_: Exception) { /* best-effort */ }
        }
    }

    // Retries creating local playlists that haven't been synced to the server yet
    // (up to 3 attempts), adopting an existing same-named server playlist instead
    // of creating a duplicate if one is found.
    suspend fun retryPendingCreates(serverPlaylists: List<ServerPlaylist>) {
        for (p in load()) {
            if (p.serverId != null || !p.createPending || p.createAttempts >= 3) continue
            val existing = serverPlaylists.find { it.name == p.name }
            if (existing != null) {
                mutate {
                    val idx = indexOfFirst { it.id == p.id }
                    if (idx >= 0) set(idx, get(idx).copy(serverId = existing.id, createPending = false))
                }
                continue
            }
            syncCreate(p)
        }
    }
}
