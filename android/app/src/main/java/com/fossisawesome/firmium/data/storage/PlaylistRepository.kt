package com.fossisawesome.firmium.data.storage

import com.fossisawesome.firmium.data.model.Playlist
import com.fossisawesome.firmium.data.model.Song
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import java.util.UUID

// Persists playlists as a JSON array in DataStore.
// Mirrors the playlists custom store from stores.js (localStorage-backed).
class PlaylistRepository(private val prefs: AppPreferences) {

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
        return p
    }

    suspend fun delete(id: String) = mutate { removeIf { it.id == id } }

    suspend fun rename(id: String, name: String) = mutate {
        val idx = indexOfFirst { it.id == id }
        if (idx >= 0) set(idx, get(idx).copy(name = name))
    }

    // Appends tracks, skipping duplicates by song id.
    suspend fun addTracks(id: String, songs: List<Song>) = mutate {
        val idx = indexOfFirst { it.id == id }
        if (idx < 0) return@mutate
        val existing = get(idx)
        val existingIds = existing.tracks.map { it.id }.toSet()
        set(idx, existing.copy(tracks = existing.tracks + songs.filter { it.id !in existingIds }))
    }

    suspend fun removeTrack(id: String, trackId: String) = mutate {
        val idx = indexOfFirst { it.id == id }
        if (idx < 0) return@mutate
        val existing = get(idx)
        set(idx, existing.copy(tracks = existing.tracks.filter { it.id != trackId }))
    }
}
