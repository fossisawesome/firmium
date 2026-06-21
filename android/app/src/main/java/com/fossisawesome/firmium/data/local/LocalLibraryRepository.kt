package com.fossisawesome.firmium.data.local

import android.content.ContentUris
import android.content.Context
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Artist
import com.fossisawesome.firmium.data.model.ArtistDetail
import com.fossisawesome.firmium.data.model.Song
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import java.security.MessageDigest

// Local-library counterpart to ApiClient, backed by files under Music/Firmium. Used by
// LibraryViewModel when AuthManager.isAuthenticated is false, so the same Album/Artist/Song
// UI works offline against locally stored/downloaded tracks.
//
// Mirrors local_library.rs (desktop): MediaStore is scanned for audio files under
// Music/Firmium, tags read via MediaMetadataRetriever, and results grouped into the same
// Album/Artist/Song shapes with `local:<hash>` IDs.
class LocalLibraryRepository(private val context: Context) {

    private data class Cache(
        val albums: List<Album>,
        val artists: List<Artist>,
        val songsByAlbum: Map<String, List<Song>>,
        val albumsByArtist: Map<String, List<Album>>,
        val artistNames: Map<String, String>,
        val allSongs: List<Song>,
        val trackUris: Map<String, Uri>,
    )

    @Volatile
    private var cache: Cache? = null

    fun invalidate() {
        cache = null
    }

    private suspend fun ensureScanned(): Cache =
        cache ?: withContext(Dispatchers.IO) { scan().also { cache = it } }

    // Triggers a background scan without exposing the internal Cache type.
    // Called from FirmiumApplication on startup so the local library is ready
    // before the user starts playing tracks.
    suspend fun prewarm() { ensureScanned() }

    suspend fun getAlbums(): List<Album> = ensureScanned().albums

    suspend fun getArtists(): List<Artist> = ensureScanned().artists

    suspend fun getAlbumDetail(albumId: String): Album {
        val c = ensureScanned()
        val album = c.albums.find { it.id == albumId } ?: error("Album not found")
        return album.copy(tracks = c.songsByAlbum[albumId] ?: emptyList())
    }

    suspend fun getArtistDetail(artistId: String): ArtistDetail {
        val c = ensureScanned()
        val name = c.artistNames[artistId] ?: error("Artist not found")
        val artist = c.artists.find { it.id == artistId } ?: Artist(artistId, name, 0, null)
        return ArtistDetail(artist, c.albumsByArtist[artistId] ?: emptyList(), null, null)
    }

    suspend fun search(query: String): ApiClient.SearchResults {
        val c = ensureScanned()
        val q = query.trim().lowercase()
        if (q.isEmpty()) return ApiClient.SearchResults(emptyList(), emptyList())
        val songs = c.allSongs.filter {
            it.title.lowercase().contains(q) || it.artist.lowercase().contains(q) || it.album.lowercase().contains(q)
        }
        val albums = c.albums.filter {
            it.name.lowercase().contains(q) || it.artist.lowercase().contains(q)
        }
        return ApiClient.SearchResults(songs, albums)
    }

    suspend fun getRecentAlbums(size: Int = 12): List<Album> = ensureScanned().albums.takeLast(size).reversed()

    suspend fun getRandomAlbums(size: Int = 12): List<Album> = ensureScanned().albums.shuffled().take(size)

    suspend fun getNewestAlbums(size: Int = 100): List<Album> = ensureScanned().albums.takeLast(size).reversed()

    // Finds a locally-downloaded song matching a server song by title + album (case-insensitive).
    // Used by PlayerViewModel to prefer the local copy over streaming, and by DownloadManager
    // to skip re-downloading tracks already on disk.
    suspend fun findLocalMatch(title: String, artist: String, album: String): Song? {
        val c = ensureScanned()
        return c.allSongs.firstOrNull { local ->
            local.title.equals(title, ignoreCase = true) &&
            (local.album.equals(album, ignoreCase = true) ||
             local.artist.equals(artist, ignoreCase = true))
        }
    }

    // True if this song has a local copy (already downloaded). `local:` songs are always local.
    suspend fun isDownloaded(song: Song): Boolean =
        song.id.startsWith("local:") || findLocalMatch(song.title, song.artist, song.album) != null

    // Of the given songs, returns the set of ids that have a local copy. Scans once, then matches
    // in memory — used to mark per-track and whole-album/playlist downloaded state in the UI.
    suspend fun downloadedIds(songs: List<Song>): Set<String> {
        if (songs.isEmpty()) return emptySet()
        val locals = ensureScanned().allSongs
        return songs.filter { song ->
            song.id.startsWith("local:") || locals.any { local ->
                local.title.equals(song.title, ignoreCase = true) &&
                (local.album.equals(song.album, ignoreCase = true) ||
                 local.artist.equals(song.artist, ignoreCase = true))
            }
        }.map { it.id }.toSet()
    }

    // Resolves a `local:<hash>` song id to its MediaStore content URI, for ExoPlayer.
    suspend fun getTrackUri(songId: String): Uri? = ensureScanned().trackUris[songId]

    // Extracts an album's embedded cover art to cacheDir/local_covers/<albumId>.jpg and
    // returns a `file://` URI string, or null if no track in the album has embedded art.
    private fun extractCoverArt(albumId: String, sourceUri: Uri): String? {
        val cacheDir = File(context.cacheDir, "local_covers").apply { mkdirs() }
        val file = File(cacheDir, "${albumId.removePrefix("local:")}.jpg")
        if (file.exists()) return Uri.fromFile(file).toString()
        val retriever = MediaMetadataRetriever()
        return try {
            retriever.setDataSource(context, sourceUri)
            val art = retriever.embeddedPicture ?: return null
            FileOutputStream(file).use { it.write(art) }
            Uri.fromFile(file).toString()
        } catch (_: Exception) {
            null
        } finally {
            retriever.release()
        }
    }

    // ── Scanning ─────────────────────────────────────────────────────────────────

    private data class RawTrack(
        val uri: Uri,
        val title: String,
        val artist: String,
        val albumArtist: String,
        val album: String,
        val track: Int?,
        val year: Int?,
        val genre: String?,
        val duration: Int,
        val bitRate: Int?,
        val suffix: String?,
        val hasArt: Boolean,
    )

    private fun queryTracks(): List<RawTrack> {
        val collection = MediaStore.Audio.Media.EXTERNAL_CONTENT_URI
        val projection = mutableListOf(
            MediaStore.Audio.Media._ID,
            MediaStore.Audio.Media.TITLE,
            MediaStore.Audio.Media.ARTIST,
            MediaStore.Audio.Media.ALBUM,
            MediaStore.Audio.Media.ALBUM_ARTIST,
            MediaStore.Audio.Media.TRACK,
            MediaStore.Audio.Media.YEAR,
            MediaStore.Audio.Media.DURATION,
            MediaStore.Audio.Media.MIME_TYPE,
        )
        val selection: String
        val selectionArgs: Array<String>
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            projection += MediaStore.Audio.Media.RELATIVE_PATH
            projection += MediaStore.Audio.Media.BITRATE
            selection = "${MediaStore.Audio.Media.RELATIVE_PATH} LIKE ?"
            selectionArgs = arrayOf("Music/Firmium%")
        } else {
            projection += MediaStore.Audio.Media.DATA
            val musicDir = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_MUSIC)
            selection = "${MediaStore.Audio.Media.DATA} LIKE ?"
            selectionArgs = arrayOf("${musicDir.absolutePath}/Firmium%")
        }

        val tracks = mutableListOf<RawTrack>()
        context.contentResolver.query(collection, projection.toTypedArray(), selection, selectionArgs, null)?.use { cursor ->
            val idCol = cursor.getColumnIndexOrThrow(MediaStore.Audio.Media._ID)
            val titleCol = cursor.getColumnIndexOrThrow(MediaStore.Audio.Media.TITLE)
            val artistCol = cursor.getColumnIndexOrThrow(MediaStore.Audio.Media.ARTIST)
            val albumCol = cursor.getColumnIndexOrThrow(MediaStore.Audio.Media.ALBUM)
            val albumArtistCol = cursor.getColumnIndexOrThrow(MediaStore.Audio.Media.ALBUM_ARTIST)
            val trackCol = cursor.getColumnIndexOrThrow(MediaStore.Audio.Media.TRACK)
            val yearCol = cursor.getColumnIndexOrThrow(MediaStore.Audio.Media.YEAR)
            val durationCol = cursor.getColumnIndexOrThrow(MediaStore.Audio.Media.DURATION)
            val mimeCol = cursor.getColumnIndexOrThrow(MediaStore.Audio.Media.MIME_TYPE)
            val bitRateCol = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) cursor.getColumnIndex(MediaStore.Audio.Media.BITRATE) else -1

            while (cursor.moveToNext()) {
                val id = cursor.getLong(idCol)
                val uri = ContentUris.withAppendedId(collection, id)
                val title = cursor.getString(titleCol) ?: continue
                val artist = cursor.getString(artistCol) ?: "Unknown Artist"
                val albumArtistRaw = cursor.getString(albumArtistCol)
                val album = cursor.getString(albumCol) ?: "Unknown Album"
                val trackRaw = cursor.getInt(trackCol)
                val year = cursor.getInt(yearCol).takeIf { it > 0 }
                val durationMs = cursor.getLong(durationCol)
                val mime = cursor.getString(mimeCol)
                val bitRate = if (bitRateCol >= 0) cursor.getInt(bitRateCol).takeIf { it > 0 }?.div(1000) else null

                // Read genre + embedded-art presence via MediaMetadataRetriever (not exposed by
                // the MediaStore audio columns).
                var genre: String? = null
                var hasArt = false
                val retriever = MediaMetadataRetriever()
                try {
                    retriever.setDataSource(context, uri)
                    genre = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_GENRE)
                    hasArt = retriever.embeddedPicture != null
                } catch (_: Exception) {
                    // Unreadable file — keep MediaStore-derived fields, skip genre/art.
                } finally {
                    retriever.release()
                }

                tracks.add(RawTrack(
                    uri = uri,
                    title = title,
                    artist = artist,
                    albumArtist = albumArtistRaw?.takeIf { it.isNotBlank() } ?: artist,
                    album = album,
                    track = if (trackRaw in 1..999) trackRaw else (trackRaw % 1000).takeIf { it > 0 },
                    year = year,
                    genre = genre,
                    duration = (durationMs / 1000).toInt(),
                    bitRate = bitRate,
                    suffix = mime?.substringAfterLast('/'),
                    hasArt = hasArt,
                ))
            }
        }
        return tracks
    }

    private fun localId(seed: String): String {
        val digest = MessageDigest.getInstance("MD5").digest(seed.toByteArray())
        return "local:" + digest.joinToString("") { "%02x".format(it) }
    }

    private fun scan(): Cache {
        val rawTracks = queryTracks()

        val songsByAlbum = LinkedHashMap<String, MutableList<Song>>()
        val albumMeta = LinkedHashMap<String, Triple<String, String, RawTrack>>() // albumId -> (name, albumArtist, firstTrack)
        val albumCoverSourceUris = HashMap<String, Uri>() // albumId -> source track uri for embedded art
        val artistAlbumIds = LinkedHashMap<String, MutableSet<String>>() // artistId -> albumIds
        val artistNames = LinkedHashMap<String, String>()
        val trackUris = HashMap<String, Uri>()
        val allSongs = mutableListOf<Song>()
        val songAlbumIds = HashMap<String, String>() // songId -> albumId, for second pass

        for (raw in rawTracks) {
            val albumArtistKey = raw.albumArtist.lowercase()
            val artistId = localId("artist:$albumArtistKey")
            val albumId = localId("album:$albumArtistKey|${raw.album.lowercase()}")
            val songId = localId("song:${raw.uri}")

            trackUris[songId] = raw.uri
            songAlbumIds[songId] = albumId
            artistNames[artistId] = raw.albumArtist
            artistAlbumIds.getOrPut(artistId) { mutableSetOf() }.add(albumId)

            val existing = albumMeta[albumId]
            if (existing == null || (!existing.third.hasArt && raw.hasArt)) {
                albumMeta[albumId] = Triple(raw.album, raw.albumArtist, raw)
            }
            if (raw.hasArt && !albumCoverSourceUris.containsKey(albumId)) albumCoverSourceUris[albumId] = raw.uri

            val song = Song(
                id = songId,
                title = raw.title,
                artist = raw.artist,
                displayArtist = null,
                album = raw.album,
                albumId = albumId,
                artistId = artistId,
                duration = raw.duration,
                track = raw.track,
                year = raw.year,
                genre = raw.genre,
                genres = raw.genre?.let { listOf(it) } ?: emptyList(),
                coverArt = null,
                size = null,
                bitRate = raw.bitRate,
                samplingRate = null,
                bitDepth = null,
                suffix = raw.suffix,
                replayGainTrack = null,
                replayGainAlbum = null,
                replayGainTrackPeak = null,
                replayGainAlbumPeak = null,
                bpm = null,
            )
            songsByAlbum.getOrPut(albumId) { mutableListOf() }.add(song)
            allSongs.add(song)
        }

        // Sort each album's tracks by track number, then title.
        for ((id, songs) in songsByAlbum) {
            songsByAlbum[id] = songs.sortedWith(compareBy({ it.track ?: Int.MAX_VALUE }, { it.title })).toMutableList()
        }

        // Extract embedded cover art per album to cacheDir/local_covers/, as file:// URIs.
        val albumCoverArt = HashMap<String, String>()
        for ((albumId, sourceUri) in albumCoverSourceUris) {
            extractCoverArt(albumId, sourceUri)?.let { albumCoverArt[albumId] = it }
        }

        // Backfill each song's coverArt with its album's cover.
        for ((id, songs) in songsByAlbum) {
            val cover = albumCoverArt[id] ?: continue
            songsByAlbum[id] = songs.map { it.copy(coverArt = cover) }.toMutableList()
        }
        for (i in allSongs.indices) {
            val albumId = songAlbumIds[allSongs[i].id] ?: continue
            albumCoverArt[albumId]?.let { allSongs[i] = allSongs[i].copy(coverArt = it) }
        }

        val albums = albumMeta.entries.map { (albumId, meta) ->
            val (name, albumArtist, firstTrack) = meta
            val tracks = songsByAlbum[albumId] ?: emptyList()
            val artistId = localId("artist:${albumArtist.lowercase()}")
            Album(
                id = albumId,
                name = name,
                artist = albumArtist,
                artistId = artistId,
                coverArt = albumCoverArt[albumId],
                songCount = tracks.size,
                duration = tracks.sumOf { it.duration },
                year = firstTrack.year,
                genre = firstTrack.genre,
                genres = firstTrack.genre?.let { listOf(it) } ?: emptyList(),
                releaseType = "Album",
                isCompilation = false,
            )
        }.sortedBy { it.name.lowercase() }

        val albumsByArtist = HashMap<String, List<Album>>()
        for ((artistId, albumIds) in artistAlbumIds) {
            albumsByArtist[artistId] = albums.filter { it.id in albumIds }
        }

        val artists = artistNames.entries.map { (artistId, name) ->
            Artist(
                id = artistId,
                name = name,
                albumCount = albumsByArtist[artistId]?.size ?: 0,
                coverArt = albumsByArtist[artistId]?.firstOrNull { it.coverArt != null }?.coverArt,
            )
        }.sortedBy { it.name.lowercase() }

        return Cache(albums, artists, songsByAlbum, albumsByArtist, artistNames, allSongs, trackUris)
    }
}
