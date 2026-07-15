package com.fossisawesome.firmium.data.model

// Mirrors the Album struct from map_albums in the Rust backend.
data class Album(
    val id: String,
    val name: String,
    val artist: String,
    val artistId: String,
    val coverArt: String?,
    val songCount: Int,
    val duration: Int,
    val year: Int?,
    val genre: String?,
    val genres: List<String>,
    val releaseType: String,
    val isCompilation: Boolean,
    // Only populated after fetching album detail (getAlbum endpoint).
    val tracks: List<Song> = emptyList(),
    val starred: Boolean = false,
)
