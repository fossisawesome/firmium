package com.fossisawesome.firmium.data.model

// Mirrors the Song struct produced by map_songs in the Rust backend.
// OpenSubsonic fields (replayGain, bpm, genres, displayArtist) are nullable
// since legacy Subsonic servers don't include them.
data class Song(
    val id: String,
    val title: String,
    val artist: String,
    val displayArtist: String?,
    val album: String,
    val albumId: String,
    val artistId: String,
    val duration: Int,
    val track: Int?,
    val year: Int?,
    val genre: String?,
    val genres: List<String>,
    val coverArt: String?,
    val size: Long?,
    val bitRate: Int?,
    val replayGainTrack: Double?,
    val replayGainAlbum: Double?,
    val bpm: Int?,
)
