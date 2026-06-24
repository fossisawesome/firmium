package com.fossisawesome.firmium.data.download

import android.content.ContentValues
import android.content.Context
import android.media.MediaScannerConnection
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.toUserError
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.local.LocalLibraryRepository
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Song
import com.google.gson.JsonParser
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File
import java.util.concurrent.TimeUnit

// Downloads tracks/albums from the OpenSubsonic server into Music/Firmium, matching the
// desktop's download_track/download_album commands (src-tauri/src/commands/downloads.rs).
// Mirrors LocalLibraryRepository's storage conventions so downloaded files show up in the
// local library scan immediately after invalidate().
class DownloadManager(
    private val context: Context,
    private val auth: AuthManager,
    private val localLibrary: LocalLibraryRepository,
) {

    private val http = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .build()

    // Application-scoped so a "download entire library" run survives navigating away from Settings.
    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    // Progress for the whole-library download, observed by the Settings screen.
    data class DownloadAllProgress(
        val running: Boolean = false,
        val done: Int = 0,
        val total: Int = 0,
        val finished: Boolean = false,
        val error: String? = null,
    )

    private val _downloadAll = MutableStateFlow(DownloadAllProgress())
    val downloadAllState: StateFlow<DownloadAllProgress> = _downloadAll.asStateFlow()

    // allowRedownload: when true (server mode) fetch the file again even if a local copy exists.
    // invalidateAfter: when false (bulk download) skip the per-track library rescan; the caller
    // invalidates once at the end to avoid an O(n^2) rescan of MediaStore.
    suspend fun downloadTrack(
        song: Song,
        format: String,
        albumArtist: String = song.artist,
        allowRedownload: Boolean = false,
        invalidateAfter: Boolean = true,
    ): Result<Unit> =
        withContext(Dispatchers.IO) {
            try {
                // Skip if this track is already in the local library — unless re-download was asked.
                if (!allowRedownload && localLibrary.findLocalMatch(song.title, song.artist, song.album) != null) {
                    return@withContext Result.success(Unit)
                }

                val url = auth.downloadUrl(song.id, format)
                val request = Request.Builder().url(url).build()
                http.newCall(request).execute().use { response ->
                    if (!response.isSuccessful) {
                        return@withContext Result.failure(Exception("HTTP ${response.code}"))
                    }
                    val body = response.body ?: return@withContext Result.failure(Exception("Empty response"))
                    val contentType = body.contentType()
                    if (contentType?.type == "application" && contentType.subtype == "json") {
                        val json = JsonParser.parseString(body.string()).asJsonObject
                        val message = json.getAsJsonObject("subsonic-response")
                            ?.getAsJsonObject("error")
                            ?.get("message")?.asString
                            ?: "Download failed"
                        return@withContext Result.failure(Exception(message))
                    }

                    val ext = if (format == "original") (song.suffix ?: "mp3") else format
                    val fileName = sanitize("%02d - %s".format(song.track ?: 0, song.title)) + ".$ext"
                    val relativeDir = "${sanitize(albumArtist)}/${sanitize(song.album)}"

                    body.byteStream().use { input ->
                        writeToFirmiumLibrary(relativeDir, fileName, mimeTypeFor(ext), input)
                    }
                }
                if (invalidateAfter) localLibrary.invalidate()
                Result.success(Unit)
            } catch (e: Exception) {
                Result.failure(e)
            }
        }

    suspend fun downloadAlbum(album: Album, format: String): Result<Unit> {
        for (track in album.tracks) {
            val result = downloadTrack(track, format, album.artist)
            if (result.isFailure) return result
        }
        return Result.success(Unit)
    }

    // Downloads every server album/track into the local library. Runs on appScope so it keeps
    // going while the user navigates away from Settings. Progress is published via downloadAllState.
    fun startDownloadAll(api: ApiClient, format: String) {
        if (_downloadAll.value.running) return
        appScope.launch {
            _downloadAll.value = DownloadAllProgress(running = true)
            try {
                // Gather every track across all albums first so the progress total is accurate.
                val pending = mutableListOf<Pair<Song, String>>()  // track to albumArtist
                for (album in api.getAlbums()) {
                    val full = runCatching { api.getAlbumDetail(album.id) }.getOrNull() ?: continue
                    for (track in full.tracks) pending.add(track to full.artist)
                }
                _downloadAll.value = DownloadAllProgress(running = true, total = pending.size)
                var done = 0
                for ((track, artist) in pending) {
                    // Skip already-local tracks; don't rescan per track (invalidate once at the end).
                    downloadTrack(track, format, artist, allowRedownload = false, invalidateAfter = false)
                    done++
                    _downloadAll.value = _downloadAll.value.copy(done = done)
                }
                localLibrary.invalidate()
                _downloadAll.value = DownloadAllProgress(done = done, total = pending.size, finished = true)
            } catch (e: Exception) {
                if (e is kotlinx.coroutines.CancellationException) throw e
                _downloadAll.value = DownloadAllProgress(error = e.toUserError().message)
            }
        }
    }

    private fun writeToFirmiumLibrary(relativeDir: String, fileName: String, mimeType: String, input: java.io.InputStream) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val values = ContentValues().apply {
                put(MediaStore.Audio.Media.DISPLAY_NAME, fileName)
                put(MediaStore.Audio.Media.MIME_TYPE, mimeType)
                put(MediaStore.Audio.Media.RELATIVE_PATH, "Music/Firmium/$relativeDir")
                put(MediaStore.Audio.Media.IS_PENDING, 1)
            }
            val resolver = context.contentResolver
            val uri = resolver.insert(MediaStore.Audio.Media.EXTERNAL_CONTENT_URI, values)
                ?: error("Failed to create media store entry")
            resolver.openOutputStream(uri)?.use { output -> input.copyTo(output) }
                ?: error("Failed to open output stream")
            values.clear()
            values.put(MediaStore.Audio.Media.IS_PENDING, 0)
            resolver.update(uri, values, null, null)
        } else {
            val musicDir = Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_MUSIC)
            val dir = File(musicDir, "Firmium/$relativeDir")
            dir.mkdirs()
            val file = File(dir, fileName)
            file.outputStream().use { output -> input.copyTo(output) }
            MediaScannerConnection.scanFile(context, arrayOf(file.absolutePath), null, null)
        }
    }

    private fun sanitize(name: String): String =
        name.replace(Regex("[/\\\\:*?\"<>|]"), "_").trim()

    private fun mimeTypeFor(ext: String): String = when (ext.lowercase()) {
        "mp3" -> "audio/mpeg"
        "flac" -> "audio/flac"
        "wav" -> "audio/wav"
        "opus" -> "audio/opus"
        "ogg" -> "audio/ogg"
        "m4a", "aac" -> "audio/mp4"
        else -> "audio/mpeg"
    }
}
