package com.fossisawesome.firmium.data.model

// Mirrors getPlaylists/getPlaylist responses (OpenSubsonic). Equivalent to
// ServerPlaylist / PlaylistTracks in desktop src/lib/api.ts.
data class ServerPlaylist(
    val id: String,
    val name: String,
    val comment: String? = null,
    val songCount: Int = 0,
    val coverArt: String? = null,
)

data class ServerPlaylistTracks(
    val id: String,
    val name: String,
    val comment: String,
    val songCount: Int,
    val tracks: List<Song>,
)
