package com.fossisawesome.firmium.data.db

import com.fossisawesome.firmium.data.model.Song
import com.google.gson.GsonBuilder
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.time.LocalDate

// ── UI models assembled from DAO projections ─────────────────────────────────
data class RecapTimeOfDay(val morning: Int, val afternoon: Int, val evening: Int, val night: Int)

data class RecapStats(
    val totalPlays: Int,
    val totalSeconds: Long,
    val topTracks: List<TrackStat>,
    val topArtists: List<ArtistStat>,
    val topAlbums: List<AlbumStat>,
    val topGenre: GenreStat?,
    val timeOfDay: RecapTimeOfDay,
    val byDayOfWeek: List<Int>,   // size 7, index 0 = Sunday
    val biggestDiscovery: DiscoveryStat?,
    val daysActive: Int,
    val longestStreak: Int,
)

data class PlayHistorySummary(
    val totalPlays: Int,
    val totalSeconds: Long,
    val uniqueTracks: Int,
    val uniqueArtists: Int,
    val uniqueAlbums: Int,
)

// Local play-history store access. All reads aggregate in SQL; no server calls.
class PlayHistoryRepository(private val dao: PlayDao) {

    suspend fun record(song: Song, durationPlayedSecs: Int) {
        val genre = song.genres.firstOrNull() ?: song.genre
        dao.insert(
            PlayEntity(
                trackId = song.id,
                trackTitle = song.title,
                artistId = song.artistId,
                artistName = song.displayArtist ?: song.artist,
                albumId = song.albumId,
                albumName = song.album,
                coverArtId = song.coverArt,
                genre = genre,
                bpm = song.bpm,
                timestamp = System.currentTimeMillis() / 1000,
                durationPlayed = durationPlayedSecs,
            )
        )
    }

    suspend fun recap(from: Long, to: Long): RecapStats {
        val hours = dao.byHour(from, to)
        val tod = RecapTimeOfDay(
            morning = hours.filter { it.hour in 5..11 }.sumOf { it.count },
            afternoon = hours.filter { it.hour in 12..16 }.sumOf { it.count },
            evening = hours.filter { it.hour in 17..20 }.sumOf { it.count },
            night = hours.filter { it.hour < 5 || it.hour > 20 }.sumOf { it.count },
        )
        val dow = IntArray(7)
        dao.byDayOfWeek(from, to).forEach { if (it.dow in 0..6) dow[it.dow] = it.count }
        val days = dao.activeDays(from, to)

        return RecapStats(
            totalPlays = dao.playCount(from, to),
            totalSeconds = dao.totalSeconds(from, to),
            topTracks = dao.topTracks(from, to, 10),
            topArtists = dao.topArtists(from, to, 10),
            topAlbums = dao.topAlbums(from, to, 10),
            topGenre = dao.topGenre(from, to),
            timeOfDay = tod,
            byDayOfWeek = dow.toList(),
            biggestDiscovery = dao.biggestDiscovery(from, to),
            daysActive = days.size,
            longestStreak = longestStreak(days),
        )
    }

    suspend fun summary(): PlayHistorySummary = PlayHistorySummary(
        totalPlays = dao.totalPlays(),
        totalSeconds = dao.totalSecondsAll(),
        uniqueTracks = dao.uniqueTracks(),
        uniqueArtists = dao.uniqueArtists(),
        uniqueAlbums = dao.uniqueAlbums(),
    )

    suspend fun exportCsv(): String = withContext(Dispatchers.Default) {
        val sb = StringBuilder(
            "track_id,track_title,artist_id,artist_name,album_id,album_name,genre,bpm,timestamp,duration_played\n"
        )
        for (p in dao.allPlays()) {
            sb.append(listOf(
                csv(p.trackId), csv(p.trackTitle), csv(p.artistId), csv(p.artistName),
                csv(p.albumId), csv(p.albumName), csv(p.genre),
                p.bpm?.toString() ?: "", p.timestamp.toString(), p.durationPlayed?.toString() ?: "",
            ).joinToString(",")).append('\n')
        }
        sb.toString()
    }

    suspend fun exportJson(): String = withContext(Dispatchers.Default) {
        GsonBuilder().setPrettyPrinting().create().toJson(dao.allPlays())
    }

    private fun csv(v: String?): String {
        val s = v ?: ""
        return if (s.any { it == ',' || it == '"' || it == '\n' || it == '\r' }) {
            "\"" + s.replace("\"", "\"\"") + "\""
        } else s
    }

    // Longest run of consecutive calendar days from a sorted list of "yyyy-MM-dd".
    private fun longestStreak(days: List<String>): Int {
        if (days.isEmpty()) return 0
        var longest = 1
        var current = 1
        var prev: Long? = null
        for (d in days) {
            val epochDay = runCatching { LocalDate.parse(d).toEpochDay() }.getOrNull() ?: continue
            if (prev != null && epochDay == prev + 1) current++ else current = 1
            if (current > longest) longest = current
            prev = epochDay
        }
        return longest
    }
}
