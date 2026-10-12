package com.fossisawesome.firmium.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query

// Aggregation lives in SQL (mirrors the desktop rusqlite queries).
@Dao
interface PlayDao {

    @Insert
    suspend fun insert(play: PlayEntity)

    // ── Summary (whole history) ──────────────────────────────────────────────
    @Query("SELECT COUNT(*) FROM plays")
    suspend fun totalPlays(): Int

    @Query("SELECT COALESCE(SUM(durationPlayed), 0) FROM plays")
    suspend fun totalSecondsAll(): Long

    @Query("SELECT COUNT(DISTINCT trackId) FROM plays")
    suspend fun uniqueTracks(): Int

    @Query("SELECT COUNT(DISTINCT artistName) FROM plays WHERE artistName IS NOT NULL")
    suspend fun uniqueArtists(): Int

    @Query("SELECT COUNT(DISTINCT albumName) FROM plays WHERE albumName IS NOT NULL")
    suspend fun uniqueAlbums(): Int

    @Query("SELECT * FROM plays ORDER BY timestamp DESC")
    suspend fun allPlays(): List<PlayEntity>
}
