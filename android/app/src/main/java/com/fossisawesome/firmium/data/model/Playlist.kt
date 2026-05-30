package com.fossisawesome.firmium.data.model

// Client-side playlist stored in DataStore.
// Firmium manages playlists locally; they are not synced to the Subsonic server.
data class Playlist(
    val id: String,
    val name: String,
    val tracks: List<Song> = emptyList(),
    val createdAt: Long = System.currentTimeMillis(),
)
