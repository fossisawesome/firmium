package com.fossisawesome.firmium

import android.app.Application
import coil.Coil
import coil.ImageLoader
import coil.disk.DiskCache
import coil.memory.MemoryCache
import com.fossisawesome.firmium.audio.AudioPlayer
import com.fossisawesome.firmium.audio.NowPlayingController
import com.fossisawesome.firmium.audio.PlaybackController
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.download.DownloadManager
import com.fossisawesome.firmium.data.local.LocalLibraryRepository
import com.fossisawesome.firmium.data.storage.AppPreferences
import com.fossisawesome.firmium.data.storage.PlaylistRepository
import com.fossisawesome.firmium.data.storage.SecureStorage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch
import okio.Path.Companion.toOkioPath
import java.io.File

// Manual DI container — holds app-wide singletons shared across ViewModels.
class FirmiumApplication : Application() {

    val prefs by lazy { AppPreferences(this) }
    val secureStorage by lazy { SecureStorage(this) }
    val auth by lazy { AuthManager(secureStorage, prefs) }
    val api by lazy { ApiClient(auth) }
    val localLibrary by lazy { LocalLibraryRepository(this) }
    val downloadManager by lazy { DownloadManager(this, auth, localLibrary) }
    val playlists by lazy { PlaylistRepository(prefs, api) }
    val audioPlayer by lazy { AudioPlayer(this) }
    val nowPlaying by lazy { NowPlayingController(this) }
    // App-scoped playback orchestration shared by the phone UI (PlayerViewModel) and Android Auto
    // (FirmiumMediaBrowserService), so the car can browse and play without an Activity present.
    val playback by lazy { PlaybackController(audioPlayer, nowPlaying, api, auth, localLibrary, prefs, playlists) }

    override fun onCreate() {
        super.onCreate()
        // Configure Coil with a persistent disk cache and a larger memory cache so cover art
        // survives navigation and stays loaded for the entire session.
        val imageLoader = ImageLoader.Builder(this)
            .memoryCache {
                MemoryCache.Builder(this)
                    .maxSizePercent(0.30)  // 30% of available heap (default is 20%)
                    .build()
            }
            .diskCache {
                DiskCache.Builder()
                    .directory(File(cacheDir, "coil_images").toOkioPath())
                    .maxSizeBytes(256L * 1024 * 1024)  // 256 MB disk cache
                    .build()
            }
            .crossfade(true)
            .build()
        Coil.setImageLoader(imageLoader)

        // Pre-scan local files so PlayerViewModel can prefer downloaded tracks over streaming
        // and DownloadManager can skip already-downloaded songs — even in server mode.
        @Suppress("OPT_IN_USAGE")
        GlobalScope.launch(Dispatchers.IO) {
            try { localLibrary.prewarm() } catch (_: Exception) {}
        }
    }
}
