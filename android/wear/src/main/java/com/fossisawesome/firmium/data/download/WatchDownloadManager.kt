package com.fossisawesome.firmium.data.download

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.*
import androidx.datastore.preferences.preferencesDataStore
import com.fossisawesome.firmium.data.api.WatchAuthManager
import com.fossisawesome.firmium.data.model.Song
import com.fossisawesome.firmium.data.storage.WatchPreferences
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.File
import java.io.IOException

private val Context.watchDownloadsDataStore: DataStore<Preferences> by preferencesDataStore("firmium_watch_downloads")

// Downloads tracks to app-private storage for offline playback. The manifest (list of downloaded
// Song objects, not just ids) is persisted as Gson JSON in its own DataStore file — storing full
// Song objects means the Downloads screen can render titles/artists without a network round trip,
// which matters since offline playback is the whole point of this feature.
class WatchDownloadManager(context: Context, private val auth: WatchAuthManager, private val prefs: WatchPreferences) {

    private val downloadsDir = File(context.filesDir, "downloads").apply { mkdirs() }
    private val store = context.watchDownloadsDataStore
    private val gson = Gson()
    private val http = OkHttpClient()

    private companion object {
        val DOWNLOADED_SONGS_JSON = stringPreferencesKey("downloaded_songs_json")
    }

    val downloadedSongs: Flow<List<Song>> = store.data.map { prefs ->
        val json = prefs[DOWNLOADED_SONGS_JSON] ?: "[]"
        try {
            gson.fromJson<List<Song>>(json, object : TypeToken<List<Song>>() {}.type)
        } catch (_: Exception) {
            emptyList()
        }
    }

    private fun localFile(songId: String): File = File(downloadsDir, songId)

    suspend fun localPathFor(songId: String): String? {
        val file = localFile(songId)
        return if (file.exists()) file.absolutePath else null
    }

    suspend fun downloadTrack(song: Song): Result<Unit> = withContext(Dispatchers.IO) {
        try {
            val format = prefs.downloadFormat.first()
            val url = auth.downloadUrl(song.id, format)
            val request = Request.Builder().url(url).build()
            http.newCall(request).execute().use { response ->
                if (!response.isSuccessful) return@withContext Result.failure(IOException("HTTP ${response.code}"))
                val body = response.body ?: return@withContext Result.failure(IOException("Empty response body"))
                localFile(song.id).outputStream().use { out -> body.byteStream().copyTo(out) }
            }
            addToManifest(song)
            Result.success(Unit)
        } catch (e: Exception) {
            localFile(song.id).delete()
            Result.failure(e)
        }
    }

    // Skips tracks already downloaded. Sequential, not parallel — acceptable for
    // album/playlist-sized batches; stops at the first failure.
    suspend fun downloadTracks(songs: List<Song>): Result<Unit> {
        val alreadyDownloaded = downloadedSongs.first().map { it.id }.toSet()
        for (song in songs) {
            if (song.id !in alreadyDownloaded) {
                downloadTrack(song).onFailure { return Result.failure(it) }
            }
        }
        return Result.success(Unit)
    }

    suspend fun deleteDownload(songId: String) {
        localFile(songId).delete()
        val remaining = downloadedSongs.first().filter { it.id != songId }
        store.edit { it[DOWNLOADED_SONGS_JSON] = gson.toJson(remaining) }
    }

    suspend fun totalStorageBytes(): Long =
        downloadedSongs.first().sumOf { localFile(it.id).length() }

    private suspend fun addToManifest(song: Song) {
        val current = downloadedSongs.first()
        if (current.none { it.id == song.id }) {
            store.edit { it[DOWNLOADED_SONGS_JSON] = gson.toJson(current + song) }
        }
    }
}
