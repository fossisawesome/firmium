package com.fossisawesome.firmium

import android.app.Application
import coil.Coil
import coil.ImageLoader
import coil.disk.DiskCache
import coil.memory.MemoryCache
import com.fossisawesome.firmium.audio.AudioPlayer
import com.fossisawesome.firmium.audio.NowPlayingController
import com.fossisawesome.firmium.audio.PlaybackController
import com.fossisawesome.firmium.data.ErrorBus
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.db.FirmiumDatabase
import com.fossisawesome.firmium.data.db.PlayHistoryRepository
import com.fossisawesome.firmium.data.download.DownloadManager
import com.fossisawesome.firmium.data.local.LocalLibraryRepository
import com.fossisawesome.firmium.data.podcast.PodcastRepository
import com.fossisawesome.firmium.data.storage.AppPreferences
import com.fossisawesome.firmium.data.storage.PlaylistRepository
import com.fossisawesome.firmium.data.storage.SecureStorage
import com.fossisawesome.firmium.wear.WearStateSync
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import okio.Path.Companion.toOkioPath
import java.io.File

// Manual DI container — holds app-wide singletons shared across ViewModels.
class FirmiumApplication : Application() {

    val errors by lazy { ErrorBus() }
    val prefs by lazy { AppPreferences(this) }
    val secureStorage by lazy { SecureStorage(this) }
    val auth by lazy { AuthManager(secureStorage, prefs) }
    val api by lazy { ApiClient(auth) }
    val localLibrary by lazy { LocalLibraryRepository(this) }
    val downloadManager by lazy { DownloadManager(this, auth, localLibrary) }
    val playlists by lazy { PlaylistRepository(prefs, api) }
    // Local play-history store (Room) — powers Stats Export and Firmium Recap.
    val playHistory by lazy { PlayHistoryRepository(FirmiumDatabase.get(this).playDao()) }
    // Local-only podcast subscriptions (RSS, client-side — Navidrome has no podcast API).
    val podcasts by lazy { PodcastRepository(FirmiumDatabase.get(this).podcastDao()) }
    val audioPlayer by lazy { AudioPlayer(this) }
    val nowPlaying by lazy { NowPlayingController(this) }
    // App-scoped playback orchestration shared by the phone UI (PlayerViewModel) and Android Auto
    // (FirmiumMediaBrowserService), so the car can browse and play without an Activity present.
    val playback by lazy { PlaybackController(audioPlayer, nowPlaying, api, auth, localLibrary, prefs, playlists, secureStorage, playHistory) }
    // Mirrors now-playing state to a paired Wear OS watch and applies its transport commands.
    val wearSync by lazy { WearStateSync(this) }
    // App-lifetime scope for background work (cancelable, unlike GlobalScope).
    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

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

        // Begin mirroring playback state to a paired watch (also forces PlaybackController to
        // initialize so the watch's transport commands have a controller to drive).
        wearSync.start()

        // Pre-scan local files so PlayerViewModel can prefer downloaded tracks over streaming
        // and DownloadManager can skip already-downloaded songs — even in server mode.
        appScope.launch {
            try { localLibrary.prewarm() } catch (e: Exception) { Log.w("Firmium", "prewarm failed", e) }
        }
    }
}
