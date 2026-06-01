package com.fossisawesome.firmium.data.api

import com.fossisawesome.firmium.data.model.*
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.logging.HttpLoggingInterceptor
import java.util.concurrent.TimeUnit

// OpenSubsonic REST client. Mirrors the Api object from api.js.
// All endpoints return parsed domain models; raw JSON parsing is contained here.
class ApiClient(private val auth: AuthManager) {

    private val http = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .addInterceptor(HttpLoggingInterceptor().apply {
            level = HttpLoggingInterceptor.Level.BASIC
        })
        .build()

    // ── Core fetch ─────────────────────────────────────────────────────────────

    private suspend fun fetch(action: String, params: Map<String, String> = emptyMap()): JsonObject {
        val url = auth.buildUrl(action, params)
        return withContext(Dispatchers.IO) {
            val response = http.newCall(Request.Builder().url(url).build()).execute()
            val body = response.body?.string() ?: error("Empty response from $action")
            val root = JsonParser.parseString(body).asJsonObject
            val data = root.getAsJsonObject("subsonic-response")
            if (data.get("status").asString != "ok") {
                val code = data.getAsJsonObject("error")?.get("code")?.asInt
                val msg = data.getAsJsonObject("error")?.get("message")?.asString
                if (code == 40 || code == 41) throw SessionExpiredException()
                error("Subsonic error $code: $msg")
            }
            data
        }
    }

    // ── Albums ─────────────────────────────────────────────────────────────────

    // Fetches alphabetical album list with up to 500 entries per page.
    suspend fun getAlbums(): List<Album> {
        val results = mutableListOf<Album>()
        var offset = 0
        while (true) {
            val data = fetch("getAlbumList2", mapOf(
                "type" to "alphabeticalByName",
                "size" to "500",
                "offset" to offset.toString(),
            ))
            val albums = data.getAsJsonObject("albumList2")
                ?.getAsJsonArray("album")
                ?.map { parseAlbum(it.asJsonObject) }
                ?: break
            results.addAll(albums)
            if (albums.size < 500) break
            offset += 500
        }
        return results
    }

    suspend fun getRecentAlbums(size: Int = 12): List<Album> {
        val data = fetch("getAlbumList2", mapOf("type" to "recent", "size" to size.toString()))
        return data.getAsJsonObject("albumList2")
            ?.getAsJsonArray("album")
            ?.map { parseAlbum(it.asJsonObject) }
            ?: emptyList()
    }

    suspend fun getRandomAlbums(size: Int = 12): List<Album> {
        val data = fetch("getAlbumList2", mapOf("type" to "random", "size" to size.toString()))
        return data.getAsJsonObject("albumList2")
            ?.getAsJsonArray("album")
            ?.map { parseAlbum(it.asJsonObject) }
            ?: emptyList()
    }

    suspend fun getNewestAlbums(size: Int = 100): List<Album> {
        val data = fetch("getAlbumList2", mapOf("type" to "newest", "size" to size.toString()))
        return data.getAsJsonObject("albumList2")
            ?.getAsJsonArray("album")
            ?.map { parseAlbum(it.asJsonObject) }
            ?: emptyList()
    }

    // Fetches a full album with its track list.
    suspend fun getAlbumDetail(albumId: String): Album {
        val data = fetch("getAlbum", mapOf("id" to albumId))
        val albumObj = data.getAsJsonObject("album")
        val tracks = albumObj.getAsJsonArray("song")
            ?.map { parseSong(it.asJsonObject) }
            ?: emptyList()
        return parseAlbum(albumObj).copy(tracks = tracks)
    }

    // ── Artists ────────────────────────────────────────────────────────────────

    // Fetches all artists, flattening the index letter grouping.
    suspend fun getArtists(): List<Artist> {
        val data = fetch("getArtists")
        return data.getAsJsonObject("artists")
            ?.getAsJsonArray("index")
            ?.flatMap { idx ->
                idx.asJsonObject.getAsJsonArray("artist")
                    ?.map { parseArtist(it.asJsonObject) }
                    ?: emptyList()
            }
            ?: emptyList()
    }

    // Fetches artist albums and bio. Returns null for bio fields if unavailable.
    suspend fun getArtistDetail(artistId: String): ArtistDetail = coroutineScope {
        // Fire getArtist and getArtistInfo2 concurrently — both only need the artistId.
        val artistDeferred = async { fetch("getArtist", mapOf("id" to artistId)) }
        val infoDeferred = async {
            try { fetch("getArtistInfo2", mapOf("id" to artistId)) } catch (_: Exception) { null }
        }

        val artistData = artistDeferred.await()
        val artistObj = artistData.getAsJsonObject("artist")
        val artist = parseArtist(artistObj)
        val albums = artistObj.getAsJsonArray("album")
            ?.map { parseAlbum(it.asJsonObject) }
            ?: emptyList()

        // Attempt to fetch bio from getArtistInfo2 (may fail on servers without Last.fm integration).
        val info = infoDeferred.await()?.getAsJsonObject("artistInfo2")
        val bio = info?.get("biography")?.asString
        val imageUrl = info?.get("largeImageUrl")?.asString
            ?: info?.get("mediumImageUrl")?.asString

        ArtistDetail(artist, albums, bio, imageUrl)
    }

    // ── Search ─────────────────────────────────────────────────────────────────

    data class SearchResults(val songs: List<Song>, val albums: List<Album>)

    suspend fun search(query: String): SearchResults {
        val data = fetch("search3", mapOf(
            "query" to query,
            "albumCount" to "40",
            "songCount" to "100",
            "artistCount" to "0",
        ))
        val result = data.getAsJsonObject("searchResult3")
        val songs = result?.getAsJsonArray("song")?.map { parseSong(it.asJsonObject) } ?: emptyList()
        val albums = result?.getAsJsonArray("album")?.map { parseAlbum(it.asJsonObject) } ?: emptyList()
        return SearchResults(songs, albums)
    }

    // ── Scrobble ───────────────────────────────────────────────────────────────

    // Fire-and-forget — mirrors the scrobble call in playback.js.
    suspend fun scrobble(songId: String, submission: Boolean) {
        try {
            fetch("scrobble", mapOf(
                "id" to songId,
                "submission" to submission.toString(),
                "time" to System.currentTimeMillis().toString(),
            ))
        } catch (_: Exception) { /* scrobble failures are non-fatal */ }
    }

    // ── Lyrics ─────────────────────────────────────────────────────────────────

    data class LyricsResult(val lines: List<LyricLine>, val synced: Boolean)
    data class LyricLine(val startMs: Long?, val text: String)

    // Tries OpenSubsonic structured lyrics, then legacy getLyrics, then LrcLib as final fallback.
    suspend fun getLyrics(songId: String, artist: String, title: String, albumName: String = "", durationSec: Int = 0, useLrclib: Boolean = true): LyricsResult? {
        // 1. OpenSubsonic extension (getLyricsBySongId) — synced timestamps preferred.
        try {
            val data = fetch("getLyricsBySongId", mapOf("id" to songId))
            val lyricsObj = data.getAsJsonObject("lyricsList")
                ?.getAsJsonArray("structuredLyrics")
                ?.firstOrNull()?.asJsonObject
            if (lyricsObj != null) {
                val synced = lyricsObj.get("synced")?.asBoolean ?: false
                val lines = lyricsObj.getAsJsonArray("line")?.map { line ->
                    val obj = line.asJsonObject
                    LyricLine(
                        startMs = if (synced) obj.get("start")?.asLong else null,
                        text = obj.get("value")?.asString ?: "",
                    )
                } ?: emptyList()
                if (lines.isNotEmpty()) return LyricsResult(lines, synced)
            }
        } catch (e: Exception) { if (e is CancellationException) throw e }

        // 2. Legacy getLyrics endpoint (Subsonic, no timestamps).
        try {
            val data = fetch("getLyrics", mapOf("artist" to artist, "title" to title))
            val text = data.getAsJsonObject("lyrics")?.get("value")?.asString
            if (!text.isNullOrBlank()) {
                val lines = text.lines().map { LyricLine(null, it) }
                if (lines.isNotEmpty()) return LyricsResult(lines, false)
            }
        } catch (e: Exception) { if (e is CancellationException) throw e }

        // 3. LrcLib — free community lyrics database, supports synced LRC format.
        if (useLrclib) {
            try {
                val url = buildString {
                    append("https://lrclib.net/api/get")
                    append("?artist_name=${java.net.URLEncoder.encode(artist, "UTF-8")}")
                    append("&track_name=${java.net.URLEncoder.encode(title, "UTF-8")}")
                    if (albumName.isNotBlank()) append("&album_name=${java.net.URLEncoder.encode(albumName, "UTF-8")}")
                    if (durationSec > 0) append("&duration=$durationSec")
                }
                val response = http.newCall(Request.Builder().url(url).build()).execute()
                val body = response.body?.string()
                if (body != null && response.isSuccessful) {
                    val obj = JsonParser.parseString(body).asJsonObject
                    val synced = obj.get("syncedLyrics")?.takeIf { !it.isJsonNull }?.asString
                    if (!synced.isNullOrBlank()) {
                        val result = parseLrc(synced)
                        if (result.lines.isNotEmpty()) return result
                    }
                    val plain = obj.get("plainLyrics")?.takeIf { !it.isJsonNull }?.asString
                    if (!plain.isNullOrBlank()) {
                        val lines = plain.lines().map { LyricLine(null, it) }
                        if (lines.isNotEmpty()) return LyricsResult(lines, false)
                    }
                }
            } catch (e: Exception) { if (e is CancellationException) throw e }
        }

        return null
    }

    // Parses LRC-format lyrics ([mm:ss.xx] text per line) into a LyricsResult.
    private fun parseLrc(lrc: String): LyricsResult {
        val lines = mutableListOf<LyricLine>()
        val re = Regex("\\[(\\d+):(\\d+(?:\\.\\d+)?)\\](.*)")
        for (raw in lrc.lines()) {
            val m = re.find(raw) ?: continue
            val min = m.groupValues[1].toLong()
            val sec = m.groupValues[2].toDouble()
            val ms = min * 60_000L + (sec * 1000).toLong()
            lines.add(LyricLine(ms, m.groupValues[3].trim()))
        }
        return LyricsResult(lines, lines.isNotEmpty())
    }

    // ── JSON parsers ───────────────────────────────────────────────────────────

    private fun parseAlbum(obj: JsonObject): Album {
        val genres = obj.getAsJsonArray("genres")
            ?.mapNotNull { it.asJsonObject.get("name")?.asString }
            ?: emptyList()
        val releaseType = inferReleaseType(
            obj.get("releaseTypes")?.let {
                if (it.isJsonArray) it.asJsonArray.mapNotNull { t -> t.asString } else null
            },
            obj.get("isCompilation")?.asBoolean ?: false,
        )
        return Album(
            id = obj.get("id").asString,
            name = obj.get("name").asString,
            artist = obj.get("artist")?.asString ?: "",
            artistId = obj.get("artistId")?.asString ?: "",
            coverArt = obj.get("coverArt")?.asString,
            songCount = obj.get("songCount")?.asInt ?: 0,
            duration = obj.get("duration")?.asInt ?: 0,
            year = obj.get("year")?.asInt,
            genre = obj.get("genre")?.asString,
            genres = genres,
            releaseType = releaseType,
            isCompilation = obj.get("isCompilation")?.asBoolean ?: false,
        )
    }

    private fun parseSong(obj: JsonObject): Song {
        val replayGain = obj.getAsJsonObject("replayGain")
        val genres = obj.getAsJsonArray("genres")
            ?.mapNotNull { it.asJsonObject.get("name")?.asString }
            ?: emptyList()
        return Song(
            id = obj.get("id").asString,
            title = obj.get("title")?.asString ?: "",
            artist = obj.get("artist")?.asString ?: "",
            displayArtist = obj.get("displayArtist")?.asString,
            album = obj.get("album")?.asString ?: "",
            albumId = obj.get("albumId")?.asString ?: "",
            artistId = obj.get("artistId")?.asString ?: "",
            duration = obj.get("duration")?.asInt ?: 0,
            track = obj.get("track")?.asInt,
            year = obj.get("year")?.asInt,
            genre = obj.get("genre")?.asString,
            genres = genres,
            coverArt = obj.get("coverArt")?.asString,
            size = obj.get("size")?.asLong,
            bitRate = obj.get("bitRate")?.asInt,
            replayGainTrack = replayGain?.get("trackGain")?.asDouble,
            replayGainAlbum = replayGain?.get("albumGain")?.asDouble,
            bpm = obj.get("bpm")?.asInt,
        )
    }

    private fun parseArtist(obj: JsonObject) = Artist(
        id = obj.get("id").asString,
        name = obj.get("name")?.asString ?: "",
        albumCount = obj.get("albumCount")?.asInt ?: 0,
        coverArt = obj.get("coverArt")?.asString,
    )

    // Mirrors infer_release_type from lib.rs.
    private fun inferReleaseType(types: List<String>?, isCompilation: Boolean): String {
        if (isCompilation) return "Compilation"
        if (types.isNullOrEmpty()) return "Album"
        val t = types.first().lowercase()
        return when {
            t.contains("single") -> "Single"
            t.contains("ep") -> "EP"
            t.contains("compilation") -> "Compilation"
            t.contains("live") -> "Live"
            t.contains("remix") -> "Remix"
            else -> "Album"
        }
    }
}

class SessionExpiredException : Exception("Session expired")
