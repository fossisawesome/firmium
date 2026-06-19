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
    val samplingRate: Int?,
    val bitDepth: Int?,
    val suffix: String?,
    val replayGainTrack: Double?,
    val replayGainAlbum: Double?,
    val replayGainTrackPeak: Double?,
    val replayGainAlbumPeak: Double?,
    val bpm: Int?,
    val userRating: Int? = null,
) {
    // Builds a "FLAC · 96 kHz · 24-bit · 1411 kbps" style summary, omitting missing parts.
    fun formatTrackInfo(): String {
        val parts = mutableListOf<String>()
        suffix?.let { parts.add(it.uppercase()) }
        samplingRate?.let {
            val khz = it / 1000.0
            val formatted = if (khz == khz.toLong().toDouble()) "${khz.toLong()}" else "%.1f".format(khz)
            parts.add("$formatted kHz")
        }
        bitDepth?.let { parts.add("$it-bit") }
        bitRate?.let { parts.add("$it kbps") }
        return parts.joinToString(" · ")
    }
}
