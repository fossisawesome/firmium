package com.fossisawesome.firmium.data.db

import com.fossisawesome.firmium.data.model.Song
import com.google.gson.GsonBuilder
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

// ── UI models assembled from DAO projections ─────────────────────────────────
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
}
