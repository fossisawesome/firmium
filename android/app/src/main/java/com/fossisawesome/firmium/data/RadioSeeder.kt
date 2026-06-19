package com.fossisawesome.firmium.data

import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.model.Song
import kotlin.math.abs

// Smart Radio seeding — shared by auto-continue, Mood Mix, and Start Radio, mirroring
// the desktop src/lib/radio.ts cascade:
//   1. Server similar tracks (sonicSimilarity, else genre/Last.fm-artist fallback)
//   2. Local library filtered by genre + BPM (±15) of the seed track
class RadioSeeder(private val api: ApiClient) {

    companion object {
        const val BATCH = 10
        const val BPM_TOLERANCE = 15
        const val POOL_SIZE = 500
    }

    enum class Energy { CHILL, MID, HIGH }

    private fun genreOf(song: Song): String? = song.genres.firstOrNull() ?: song.genre

    // Returns up to [count] tracks similar to [seed], excluding the seed and [exclude].
    suspend fun seedFrom(seed: Song, exclude: Set<String> = emptySet(), count: Int = BATCH): List<Song> {
        val skip = HashSet(exclude).apply { add(seed.id) }
        val out = mutableListOf<Song>()
        fun push(s: Song) { if (skip.add(s.id)) out.add(s) }

        // 1. Server similar tracks.
        try {
            val matches = if (api.hasExtension("sonicSimilarity"))
                api.getSonicSimilarTracks(seed.id, count * 2)
            else
                api.getSimilarTracksFallback(seed.id, seed.artistId, genreOf(seed), count * 2)
            matches.forEach { push(it.song) }
        } catch (_: Exception) { /* fall through to local filter */ }

        // 2. Local library by genre + BPM.
        if (out.size < count) {
            val seedBpm = seed.bpm
            try {
                val genre = genreOf(seed)
                val pool = if (genre != null) api.getSongsByGenre(genre, POOL_SIZE) else api.getRandomSongs(POOL_SIZE)
                pool.filter { seedBpm == null || (it.bpm != null && abs(it.bpm - seedBpm) <= BPM_TOLERANCE) }
                    .shuffled()
                    .forEach { push(it) }
            } catch (_: Exception) { /* leave whatever step 1 produced */ }
        }
        return out.take(count)
    }

    private fun inBand(bpm: Int?, energy: Energy): Boolean {
        if (bpm == null) return false
        return when (energy) {
            Energy.CHILL -> bpm < 80
            Energy.MID -> bpm in 80..120
            Energy.HIGH -> bpm > 120
        }
    }

    // Shuffled queue of library tracks matching an energy band (+ optional genre).
    suspend fun buildMoodMix(energy: Energy, genre: String? = null): List<Song> {
        val pool = if (!genre.isNullOrBlank()) api.getSongsByGenre(genre, POOL_SIZE) else api.getRandomSongs(POOL_SIZE)
        return pool.filter { inBand(it.bpm, energy) }.shuffled()
    }
}
