package com.fossisawesome.firmium.data.model

// Client-side playlist stored in DataStore, best-effort synced to the OpenSubsonic server.
data class Playlist(
    val id: String,
    val name: String,
    val tracks: List<Song> = emptyList(),
    val createdAt: Long = System.currentTimeMillis(),
    val serverId: String? = null,
    // True until the playlist is first created on the server, or until createAttempts hits the retry cap.
    val createPending: Boolean = true,
    val createAttempts: Int = 0,
)
