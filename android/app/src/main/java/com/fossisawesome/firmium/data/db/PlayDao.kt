package com.fossisawesome.firmium.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query

// Aggregation lives in SQL (mirrors the desktop rusqlite queries). Each stat
// method is windowed by [from, to) in unix seconds. SELECT aliases match the
// projection field names so Room can map them.
@Dao
interface PlayDao {

    @Insert
    suspend fun insert(play: PlayEntity)

    @Query("SELECT COUNT(*) FROM plays WHERE timestamp >= :from AND timestamp < :to")
    suspend fun playCount(from: Long, to: Long): Int

    @Query("SELECT COALESCE(SUM(durationPlayed), 0) FROM plays WHERE timestamp >= :from AND timestamp < :to")
    suspend fun totalSeconds(from: Long, to: Long): Long

    @Query(
        "SELECT trackId, trackTitle AS title, artistName AS artist, coverArtId, COUNT(*) AS count " +
            "FROM plays WHERE timestamp >= :from AND timestamp < :to " +
            "GROUP BY trackId ORDER BY count DESC, title ASC LIMIT :limit"
    )
    suspend fun topTracks(from: Long, to: Long, limit: Int): List<TrackStat>

    @Query(
        "SELECT artistName AS name, COUNT(*) AS count " +
            "FROM plays WHERE timestamp >= :from AND timestamp < :to AND artistName IS NOT NULL " +
            "GROUP BY artistName ORDER BY count DESC, name ASC LIMIT :limit"
    )
    suspend fun topArtists(from: Long, to: Long, limit: Int): List<ArtistStat>

    @Query(
        "SELECT albumName AS name, artistName AS artist, coverArtId, COUNT(*) AS count " +
            "FROM plays WHERE timestamp >= :from AND timestamp < :to AND albumName IS NOT NULL " +
            "GROUP BY albumName ORDER BY count DESC, name ASC LIMIT :limit"
    )
    suspend fun topAlbums(from: Long, to: Long, limit: Int): List<AlbumStat>

    @Query(
        "SELECT genre, COUNT(*) AS count " +
            "FROM plays WHERE timestamp >= :from AND timestamp < :to AND genre IS NOT NULL AND genre != '' " +
            "GROUP BY genre ORDER BY count DESC LIMIT 1"
    )
    suspend fun topGenre(from: Long, to: Long): GenreStat?

    @Query(
        "SELECT CAST(strftime('%H', timestamp, 'unixepoch', 'localtime') AS INTEGER) AS hour, COUNT(*) AS count " +
            "FROM plays WHERE timestamp >= :from AND timestamp < :to GROUP BY hour"
    )
    suspend fun byHour(from: Long, to: Long): List<HourCount>

    @Query(
        "SELECT CAST(strftime('%w', timestamp, 'unixepoch', 'localtime') AS INTEGER) AS dow, COUNT(*) AS count " +
            "FROM plays WHERE timestamp >= :from AND timestamp < :to GROUP BY dow"
    )
    suspend fun byDayOfWeek(from: Long, to: Long): List<DowCount>

    @Query(
        "SELECT trackId, trackTitle AS title, artistName AS artist, coverArtId, COUNT(*) AS count, MIN(timestamp) AS firstHeard " +
            "FROM plays WHERE timestamp >= :from AND timestamp < :to " +
            "GROUP BY trackId HAVING COUNT(*) > 1 ORDER BY count DESC, firstHeard DESC LIMIT 1"
    )
    suspend fun biggestDiscovery(from: Long, to: Long): DiscoveryStat?

    @Query(
        "SELECT DISTINCT date(timestamp, 'unixepoch', 'localtime') " +
            "FROM plays WHERE timestamp >= :from AND timestamp < :to ORDER BY 1 ASC"
    )
    suspend fun activeDays(from: Long, to: Long): List<String>

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

// ── Query projections ───────────────────────────────────────────────────────
data class TrackStat(val trackId: String, val title: String, val artist: String?, val coverArtId: String?, val count: Int)
data class ArtistStat(val name: String, val count: Int)
data class AlbumStat(val name: String, val artist: String?, val coverArtId: String?, val count: Int)
data class GenreStat(val genre: String, val count: Int)
data class HourCount(val hour: Int, val count: Int)
data class DowCount(val dow: Int, val count: Int)
data class DiscoveryStat(val trackId: String, val title: String, val artist: String?, val coverArtId: String?, val count: Int, val firstHeard: Long)
