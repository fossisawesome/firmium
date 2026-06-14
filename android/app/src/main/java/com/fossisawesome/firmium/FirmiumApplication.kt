package com.fossisawesome.firmium

import android.app.Application
import coil.Coil
import coil.ImageLoader
import coil.disk.DiskCache
import coil.memory.MemoryCache
import com.fossisawesome.firmium.audio.AudioPlayer
import com.fossisawesome.firmium.audio.NowPlayingController
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.download.DownloadManager
import com.fossisawesome.firmium.data.local.LocalLibraryRepository
import com.fossisawesome.firmium.data.storage.AppPreferences
import com.fossisawesome.firmium.data.storage.PlaylistRepository
import com.fossisawesome.firmium.data.storage.SecureStorage
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
    val playlists by lazy { PlaylistRepository(prefs) }
    val audioPlayer by lazy { AudioPlayer(this) }
    val nowPlaying by lazy { NowPlayingController(this) }

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
    }
}
