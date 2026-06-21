package com.fossisawesome.firmium.data.api

import com.fossisawesome.firmium.BuildConfig
import com.fossisawesome.firmium.data.model.*
import com.google.gson.JsonElement
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.logging.HttpLoggingInterceptor
import java.util.concurrent.TimeUnit

// OpenSubsonic REST client. Equivalent to the Api object in src/lib/api.ts (desktop).
// All endpoints return parsed domain models; raw JSON parsing is contained here.
class ApiClient(private val auth: AuthManager) {

    private val http = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .apply {
            // BASIC logging includes the request URL, which carries the OpenSubsonic
            // auth token (t=) and salt (s=) as query params — keep this out of release logcat.
            if (BuildConfig.DEBUG) {
                addInterceptor(HttpLoggingInterceptor().apply {
                    level = HttpLoggingInterceptor.Level.BASIC
                })
            }
        }
        .build()

    // ── Core fetch ─────────────────────────────────────────────────────────────

    // OpenSubsonic extensions advertised by the server, refreshed on every response.
    // Mirrors ConnectionState.open_subsonic_extensions on desktop.
    @Volatile
    var openSubsonicExtensions: Set<String> = emptySet()
        private set

    // Emits whenever the server rejects credentials (error 40/41). Mirrors the
    // firmium:session-expired Tauri event on desktop — collectors show the login dialog.
    private val _sessionExpired = MutableSharedFlow<Unit>(extraBufferCapacity = 1)
    val sessionExpired: SharedFlow<Unit> = _sessionExpired.asSharedFlow()

    fun hasExtension(name: String): Boolean = openSubsonicExtensions.contains(name)

    private suspend fun fetch(action: String, params: Map<String, String> = emptyMap()): JsonObject =
        fetch(action, params.toList())

    // Variant accepting a list of pairs so callers can pass repeated query params
    // (e.g. multiple songIdToAdd entries for updatePlaylist), which a Map can't represent.
    private suspend fun fetch(action: String, params: List<Pair<String, String>>): JsonObject {
        val url = auth.buildUrl(action, params)
        return withContext(Dispatchers.IO) {
            val response = http.newCall(Request.Builder().url(url).build()).execute()
            val body = response.body?.string() ?: error("Empty response from $action")
            // A misconfigured URL or reverse proxy can return HTML / non-JSON; fail with a
            // clear message instead of an opaque NullPointerException.
            val root = try { JsonParser.parseString(body).asJsonObject }
                       catch (_: Exception) { error("Invalid response from $action") }
            val data = root.getAsJsonObject("subsonic-response")
                       ?: error("Invalid response from $action")
            data.getAsJsonArray("openSubsonicExtensions")?.let { extensions ->
                openSubsonicExtensions = extensions.mapNotNull { it.asJsonObject.get("name")?.asString }.toSet()
            }
            if (data.get("status")?.asString != "ok") {
                val code = data.getAsJsonObject("error")?.get("code")?.asInt
                val msg = data.getAsJsonObject("error")?.get("message")?.asString
                if (code == 40 || code == 41) { _sessionExpired.tryEmit(Unit); throw SessionExpiredException() }
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

    // ── Playlists ──────────────────────────────────────────────────────────────

    // Returns all playlists visible to the current user.
    suspend fun getPlaylists(): List<ServerPlaylist> {
        val data = fetch("getPlaylists")
        return jsonArray(data.getAsJsonObject("playlists"), "playlist").map { parsePlaylist(it.asJsonObject) }
    }

    // Fetches a playlist's full track list from the server.
    suspend fun getPlaylistTracks(id: String): ServerPlaylistTracks {
        val data = fetch("getPlaylist", mapOf("id" to id))
        val playlist = data.getAsJsonObject("playlist") ?: JsonObject()
        return ServerPlaylistTracks(
            id = playlist.get("id")?.asString ?: "",
            name = playlist.get("name")?.asString ?: "",
            comment = playlist.get("comment")?.asString ?: "",
            songCount = playlist.get("songCount")?.asInt ?: 0,
            tracks = jsonArray(playlist, "entry").map { parseSong(it.asJsonObject) },
        )
    }

    // Some servers return a single object instead of a one-element array when a
    // collection (e.g. playlists, playlist entries) contains exactly one item.
    private fun jsonArray(obj: JsonObject?, key: String): List<JsonElement> {
        val el = obj?.get(key) ?: return emptyList()
        return if (el.isJsonArray) el.asJsonArray.toList() else listOf(el)
    }

    // Creates a new playlist on the server and returns the created playlist's metadata.
    suspend fun createPlaylist(name: String): ServerPlaylist {
        val data = fetch("createPlaylist", mapOf("name" to name))
        return parsePlaylist(data.getAsJsonObject("playlist") ?: JsonObject())
    }

    // Updates playlist metadata and/or adds/removes tracks by server-side id/index.
    suspend fun updatePlaylist(
        id: String,
        name: String? = null,
        comment: String? = null,
        songIdsToAdd: List<String> = emptyList(),
        songIndicesToRemove: List<Int> = emptyList(),
    ) {
        val params = mutableListOf("playlistId" to id)
        name?.let { params.add("name" to it) }
        comment?.let { params.add("comment" to it) }
        songIdsToAdd.forEach { params.add("songIdToAdd" to it) }
        songIndicesToRemove.forEach { params.add("songIndexToRemove" to it.toString()) }
        fetch("updatePlaylist", params)
    }

    // Deletes a playlist from the server.
    suspend fun deletePlaylist(id: String) {
        fetch("deletePlaylist", mapOf("id" to id))
    }

    private fun parsePlaylist(obj: JsonObject): ServerPlaylist = ServerPlaylist(
        id = obj.get("id")?.asString ?: "",
        name = obj.get("name")?.asString ?: "",
        comment = obj.get("comment")?.asString,
        songCount = obj.get("songCount")?.asInt ?: 0,
        coverArt = obj.get("coverArt")?.asString,
    )

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

    // ── Rating ─────────────────────────────────────────────────────────────────

    suspend fun setRating(songId: String, rating: Int) {
        try {
            fetch("setRating", mapOf("id" to songId, "rating" to rating.toString()))
        } catch (_: Exception) { /* rating failures are non-fatal */ }
    }

    // ── Playback reporting ────────────────────────────────────────────────────

    // Reports playback state/position via the playbackReport OpenSubsonic extension
    // (reportPlayback). No-op if the server hasn't advertised the extension.
    suspend fun reportPlayback(songId: String, positionMs: Long, state: String) {
        if (!hasExtension("playbackReport")) return
        try {
            fetch("reportPlayback", mapOf(
                "mediaId" to songId,
                "mediaType" to "song",
                "positionMs" to positionMs.toString(),
                "state" to state,
            ))
        } catch (_: Exception) { /* report failures are non-fatal */ }
    }

    // ── Sonic similarity ──────────────────────────────────────────────────────

    data class SimilarMatch(val song: Song, val similarity: Double)

    // Fetches audio-similar tracks via the sonicSimilarity OpenSubsonic extension
    // (getSonicSimilarTracks). Throws if the server hasn't advertised the extension,
    // so callers can hide the feature.
    suspend fun getSonicSimilarTracks(songId: String, count: Int? = null): List<SimilarMatch> {
        if (!hasExtension("sonicSimilarity")) error("sonicSimilarity not supported")
        val params = mutableMapOf("id" to songId)
        count?.let { params["count"] = it.toString() }
        val data = fetch("getSonicSimilarTracks", params)
        return data.getAsJsonArray("sonicMatch")
            ?.map {
                val obj = it.asJsonObject
                SimilarMatch(parseSong(obj.getAsJsonObject("entry")), obj.get("similarity")?.asDouble ?: 0.0)
            }
            ?: emptyList()
    }

    // Fallback "similar tracks" for servers without sonicSimilarity: matches by genre
    // (getSongsByGenre, similarity 0.55) and by Last.fm-backed similar artists
    // (getArtistInfo2 -> getTopSongs, similarity 0.45). Never throws.
    suspend fun getSimilarTracksFallback(songId: String, artistId: String?, genre: String?, count: Int = 10): List<SimilarMatch> {
        val results = mutableListOf<SimilarMatch>()
        val seenIds = mutableSetOf(songId)

        if (!genre.isNullOrBlank()) {
            try {
                val data = fetch("getSongsByGenre", mapOf("genre" to genre, "count" to (count * 2).toString()))
                data.getAsJsonObject("songsByGenre")?.getAsJsonArray("song")?.forEach {
                    val song = parseSong(it.asJsonObject)
                    if (seenIds.add(song.id)) results.add(SimilarMatch(song, 0.55))
                }
            } catch (_: Exception) { /* genre lookup is best-effort */ }
        }

        if (!artistId.isNullOrBlank()) {
            try {
                val data = fetch("getArtistInfo2", mapOf("id" to artistId, "count" to "5"))
                val similarArtists = data.getAsJsonObject("artistInfo2")?.getAsJsonArray("similarArtist") ?: emptyList()
                for (similar in similarArtists.take(3)) {
                    val name = similar.asJsonObject.get("name")?.asString ?: continue
                    try {
                        val topData = fetch("getTopSongs", mapOf("artist" to name, "count" to "2"))
                        topData.getAsJsonObject("topSongs")?.getAsJsonArray("song")?.forEach {
                            val song = parseSong(it.asJsonObject)
                            if (seenIds.add(song.id)) results.add(SimilarMatch(song, 0.45))
                        }
                    } catch (_: Exception) { /* per-artist lookup is best-effort */ }
                }
            } catch (_: Exception) { /* similar-artist lookup is best-effort */ }
        }

        return results.shuffled().take(count)
    }

    // ── Library song enumeration (Radio / Mood Mix seeding) ──────────────────────

    // Songs of a given genre via getSongsByGenre, for genre + BPM filtering in-app.
    suspend fun getSongsByGenre(genre: String, count: Int = 100): List<Song> {
        val data = fetch("getSongsByGenre", mapOf("genre" to genre, "count" to count.coerceIn(1, 500).toString()))
        return data.getAsJsonObject("songsByGenre")?.getAsJsonArray("song")
            ?.map { parseSong(it.asJsonObject) } ?: emptyList()
    }

    // Top songs for an artist (by name) via getTopSongs — used for the artist page "Songs" section.
    suspend fun getTopSongs(artistName: String, count: Int = 20): List<Song> {
        return try {
            val data = fetch("getTopSongs", mapOf("artist" to artistName, "count" to count.coerceIn(1, 50).toString()))
            data.getAsJsonObject("topSongs")?.getAsJsonArray("song")
                ?.map { parseSong(it.asJsonObject) } ?: emptyList()
        } catch (_: Exception) { emptyList() }
    }

    // Random sample of library songs via getRandomSongs (optionally genre-scoped).
    suspend fun getRandomSongs(count: Int = 100, genre: String? = null): List<Song> {
        val params = mutableMapOf("size" to count.coerceIn(1, 500).toString())
        if (!genre.isNullOrBlank()) params["genre"] = genre
        val data = fetch("getRandomSongs", params)
        return data.getAsJsonObject("randomSongs")?.getAsJsonArray("song")
            ?.map { parseSong(it.asJsonObject) } ?: emptyList()
    }

    // Names of similar artists from getArtistInfo2 (similarArtist[]), for the
    // artist-page "You might also like" section.
    suspend fun getSimilarArtists(artistId: String, count: Int = 20): List<String> {
        return try {
            val data = fetch("getArtistInfo2", mapOf("id" to artistId, "count" to count.toString()))
            data.getAsJsonObject("artistInfo2")?.getAsJsonArray("similarArtist")
                ?.mapNotNull { it.asJsonObject.get("name")?.asString } ?: emptyList()
        } catch (_: Exception) { emptyList() }
    }

    // Genre names with at least one song, for the Mood Mix genre filter.
    suspend fun getGenres(): List<String> {
        val data = fetch("getGenres")
        return data.getAsJsonObject("genres")?.getAsJsonArray("genre")
            ?.mapNotNull { it.asJsonObject.get("value")?.asString }
            ?.filter { it.isNotBlank() }
            ?: emptyList()
    }

    // Similar-artist names from Last.fm directly (artist.getSimilar), used as the
    // artist-recommendations fallback when the server returns none.
    suspend fun getLastfmSimilarArtists(artistName: String, apiKey: String): List<String> {
        if (apiKey.isBlank()) return emptyList()
        return try {
            val url = "https://ws.audioscrobbler.com/2.0/?method=artist.getsimilar" +
                "&artist=${java.net.URLEncoder.encode(artistName, "UTF-8")}" +
                "&api_key=${java.net.URLEncoder.encode(apiKey, "UTF-8")}&format=json&limit=40"
            withContext(Dispatchers.IO) {
                val resp = http.newCall(Request.Builder().url(url).build()).execute()
                val body = resp.body?.string() ?: return@withContext emptyList()
                JsonParser.parseString(body).asJsonObject
                    .getAsJsonObject("similarartists")?.getAsJsonArray("artist")
                    ?.mapNotNull { it.asJsonObject.get("name")?.asString } ?: emptyList()
            }
        } catch (_: Exception) { emptyList() }
    }

    // ── ListenBrainz ─────────────────────────────────────────────────────────────

    // Submits a single "listen" to ListenBrainz on track completion. Fire-and-forget;
    // no-op when the token is blank. Plain HTTP POST, no extra dependencies.
    suspend fun submitListenBrainz(token: String, song: Song) {
        if (token.isBlank()) return
        try {
            val trackMeta = JsonObject().apply {
                addProperty("artist_name", song.displayArtist ?: song.artist)
                addProperty("track_name", song.title)
                if (song.album.isNotBlank()) addProperty("release_name", song.album)
            }
            val listen = JsonObject().apply {
                addProperty("listened_at", System.currentTimeMillis() / 1000)
                add("track_metadata", trackMeta)
            }
            val payload = JsonObject().apply {
                addProperty("listen_type", "single")
                add("payload", com.google.gson.JsonArray().apply { add(listen) })
            }
            withContext(Dispatchers.IO) {
                val body = okhttp3.RequestBody.create(
                    "application/json; charset=utf-8".toMediaTypeOrNull(),
                    payload.toString(),
                )
                val request = Request.Builder()
                    .url("https://api.listenbrainz.org/1/submit-listens")
                    .header("Authorization", "Token $token")
                    .post(body)
                    .build()
                http.newCall(request).execute().close()
            }
        } catch (_: Exception) { /* listen submission is non-fatal */ }
    }

    // ── Lyrics ─────────────────────────────────────────────────────────────────

    data class LyricsResult(val lines: List<LyricLine>, val synced: Boolean)
    // startMs is milliseconds from track start, matching the desktop LyricLine.start field (src/lib/lyrics.ts).
    data class LyricLine(val startMs: Long?, val text: String)

    // Runs a single lyric source, swallowing failures so callers can fall through to the next source.
    private suspend fun tryFetchLyrics(source: suspend () -> LyricsResult?): LyricsResult? {
        return try {
            source()
        } catch (e: Exception) {
            if (e is CancellationException) throw e
            null
        }
    }

    // Tries OpenSubsonic structured lyrics, then legacy getLyrics, then LrcLib as final fallback.
    suspend fun getLyrics(songId: String, artist: String, title: String, albumName: String = "", durationSec: Int = 0, useLrclib: Boolean = true): LyricsResult? {
        // 1. OpenSubsonic extension (getLyricsBySongId) — synced timestamps preferred.
        tryFetchLyrics {
            val data = fetch("getLyricsBySongId", mapOf("id" to songId))
            val lyricsObj = data.getAsJsonObject("lyricsList")
                ?.getAsJsonArray("structuredLyrics")
                ?.firstOrNull()?.asJsonObject
                ?: return@tryFetchLyrics null
            val synced = lyricsObj.get("synced")?.asBoolean ?: false
            val lines = lyricsObj.getAsJsonArray("line")?.map { line ->
                val obj = line.asJsonObject
                LyricLine(
                    startMs = if (synced) obj.get("start")?.asLong else null,
                    text = obj.get("value")?.asString ?: "",
                )
            } ?: emptyList()
            if (lines.isNotEmpty()) LyricsResult(lines, synced) else null
        }?.let { return it }

        // 2. Legacy getLyrics endpoint (Subsonic, no timestamps).
        tryFetchLyrics {
            val data = fetch("getLyrics", mapOf("artist" to artist, "title" to title))
            val text = data.getAsJsonObject("lyrics")?.get("value")?.asString
            if (text.isNullOrBlank()) return@tryFetchLyrics null
            val lines = text.lines().map { LyricLine(null, it) }
            if (lines.isNotEmpty()) LyricsResult(lines, false) else null
        }?.let { return it }

        // 3. LrcLib — free community lyrics database, supports synced LRC format.
        if (useLrclib) {
            tryFetchLyrics {
                val url = buildString {
                    append("https://lrclib.net/api/get")
                    append("?artist_name=${java.net.URLEncoder.encode(artist, "UTF-8")}")
                    append("&track_name=${java.net.URLEncoder.encode(title, "UTF-8")}")
                    if (albumName.isNotBlank()) append("&album_name=${java.net.URLEncoder.encode(albumName, "UTF-8")}")
                    if (durationSec > 0) append("&duration=$durationSec")
                }
                val (body, isSuccessful) = withContext(Dispatchers.IO) {
                    val request = Request.Builder()
                        .url(url)
                        .header("Lrclib-Client", "Firmium (https://github.com/fossisawesome/firmium)")
                        .build()
                    val response = http.newCall(request).execute()
                    response.body?.string() to response.isSuccessful
                }
                if (body == null || !isSuccessful) return@tryFetchLyrics null
                val obj = JsonParser.parseString(body).asJsonObject
                val synced = obj.get("syncedLyrics")?.takeIf { !it.isJsonNull }?.asString
                if (!synced.isNullOrBlank()) {
                    val result = parseLrc(synced)
                    if (result.lines.isNotEmpty()) return@tryFetchLyrics result
                }
                val plain = obj.get("plainLyrics")?.takeIf { !it.isJsonNull }?.asString
                if (!plain.isNullOrBlank()) {
                    val lines = plain.lines().map { LyricLine(null, it) }
                    if (lines.isNotEmpty()) LyricsResult(lines, false) else null
                } else null
            }?.let { return it }
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
            samplingRate = obj.get("samplingRate")?.asInt,
            bitDepth = obj.get("bitDepth")?.asInt,
            suffix = obj.get("suffix")?.asString,
            replayGainTrack = replayGain?.get("trackGain")?.asDouble,
            replayGainAlbum = replayGain?.get("albumGain")?.asDouble,
            replayGainTrackPeak = replayGain?.get("trackPeak")?.asDouble,
            replayGainAlbumPeak = replayGain?.get("albumPeak")?.asDouble,
            bpm = obj.get("bpm")?.asInt,
            userRating = obj.get("userRating")?.asInt,
        )
    }

    private fun parseArtist(obj: JsonObject) = Artist(
        id = obj.get("id").asString,
        name = obj.get("name")?.asString ?: "",
        albumCount = obj.get("albumCount")?.asInt ?: 0,
        coverArt = obj.get("coverArt")?.asString,
    )

    // Android release-type inference. NOTE: diverges from infer_release_type in mappers.rs
    // (Title Case vs lowercase output, includes Compilation/Live/Remix, checks isCompilation
    // first, no title/songCount fallback — see effectiveType() in AlbumListScreen.kt for that layer).
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
