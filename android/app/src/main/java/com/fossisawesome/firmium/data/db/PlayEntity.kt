package com.fossisawesome.firmium.data.db

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

// One row per completed play, written from the scrobble-completion path
// (PlaybackController). Name columns are denormalized so Recap renders fully
// offline without resolving IDs against the server. Mirrors the desktop
// SQLite schema (src-tauri/src/db.rs).
@Entity(tableName = "plays", indices = [Index("timestamp")])
data class PlayEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val trackId: String,
    val trackTitle: String,
    val artistId: String?,
    val artistName: String?,
    val albumId: String?,
    val albumName: String?,
    val coverArtId: String?,
    val genre: String?,
    val bpm: Int?,
    val timestamp: Long,        // unix seconds
    val durationPlayed: Int?,   // seconds actually played
)
