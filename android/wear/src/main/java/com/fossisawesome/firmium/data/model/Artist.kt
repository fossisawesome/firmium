package com.fossisawesome.firmium.data.model

// Top-level artist entry from getArtists.
data class Artist(
    val id: String,
    val name: String,
    val albumCount: Int,
    val coverArt: String?,
)

// Full artist detail including albums, bio, and external image URL.
data class ArtistDetail(
    val artist: Artist,
    val albums: List<Album>,
    val bio: String?,
    val imageUrl: String?,
)
