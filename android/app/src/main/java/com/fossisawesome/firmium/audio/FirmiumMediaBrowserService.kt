package com.fossisawesome.firmium.audio

import android.net.Uri
import android.os.Bundle
import android.support.v4.media.MediaBrowserCompat.MediaItem
import android.support.v4.media.MediaDescriptionCompat
import androidx.media.MediaBrowserServiceCompat
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Artist
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

// Media-browser service Android Auto binds to. Exposes Firmium's library as a browse tree and
// publishes the shared MediaSessionCompat (owned by NowPlayingController) so the car's now-playing
// screen and transport controls work. Playback is driven through the app-scoped PlaybackController,
// so browsing/playing works without the phone Activity being open.
class FirmiumMediaBrowserService : MediaBrowserServiceCompat() {

    private val app get() = application as FirmiumApplication
    private val api get() = app.api
    private val auth get() = app.auth
    private val localLibrary get() = app.localLibrary
    private val playlists get() = app.playlists

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    override fun onCreate() {
        super.onCreate()
        sessionToken = app.nowPlaying.session().sessionToken
        // Force PlaybackController to initialize so it registers the session's transport listener
        // (onPlayFromMediaId/onPlayFromSearch). On a car-only cold start no Activity exists to do
        // this, so without it taps in the car would have no handler.
        app.playback
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    // The service is exported so the system media browsers (Android Auto, Wear, Assistant) can
    // bind. Restrict the browse tree to those known callers plus ourselves so an arbitrary app
    // can't enumerate the user's library; returning null denies the bind. Transport still also
    // requires the shared session.
    override fun onGetRoot(clientPackageName: String, clientUid: Int, rootHints: Bundle?): BrowserRoot? {
        if (clientPackageName != packageName && clientPackageName !in ALLOWED_BROWSER_PACKAGES) {
            return null
        }
        val extras = Bundle().apply {
            putBoolean(CONTENT_STYLE_SUPPORTED, true)
            putBoolean(SEARCH_SUPPORTED, true)
            putInt(CONTENT_STYLE_BROWSABLE_HINT, CONTENT_STYLE_GRID)
            putInt(CONTENT_STYLE_PLAYABLE_HINT, CONTENT_STYLE_LIST)
        }
        return BrowserRoot(MediaTree.ROOT, extras)
    }

    override fun onLoadChildren(parentId: String, result: Result<MutableList<MediaItem>>) {
        result.detach()
        scope.launch {
            result.sendResult(runCatching { loadChildren(parentId) }.getOrDefault(mutableListOf()))
        }
    }

    override fun onSearch(query: String, extras: Bundle?, result: Result<MutableList<MediaItem>>) {
        result.detach()
        scope.launch {
            result.sendResult(runCatching { searchItems(query) }.getOrDefault(mutableListOf()))
        }
    }

    // ── Browse tree ──────────────────────────────────────────────────────────────

    private suspend fun loadChildren(parentId: String): MutableList<MediaItem> {
        // Nothing to browse when signed out and no local library — prompt the user to sign in.
        if (!auth.isAuthenticated && localLibrary.getAlbums().isEmpty()) {
            return mutableListOf(browsable("info_sign_in", "Sign in on your phone", "Open Firmium to connect", null))
        }
        return when (val node = MediaTree.parse(parentId)) {
            MediaNode.Root -> mutableListOf(
                browsable(MediaTree.HOME, "Home", null, null),
                browsable(MediaTree.MUSIC, "Music", null, null),
                browsable(MediaTree.ARTISTS, "Artists", null, null),
                browsable(MediaTree.PLAYLISTS, "Playlists", null, null),
            )
            MediaNode.Home -> homeChildren()
            // A–Z index instead of one giant album list — the letters render instantly and only
            // the chosen letter's albums are fetched/filtered, avoiding the "loads forever" stall.
            MediaNode.Music -> musicLetters()
            is MediaNode.MusicLetter -> allAlbumsCached()
                .filter { letterBucket(it.name) == node.bucket }
                .map { browsableAlbum(it) }.toMutableList()
            MediaNode.Artists -> artists().map { browsableArtist(it) }.toMutableList()
            MediaNode.Playlists -> playlistChildren()
            is MediaNode.Album -> {
                val tracks = albumDetail(node.albumId).tracks
                val items = mutableListOf<MediaItem>()
                if (tracks.isNotEmpty()) {
                    items.add(playable(MediaTree.albumShuffleId(node.albumId), "Shuffle", "Shuffle this album", null))
                }
                tracks.forEach {
                    items.add(playable(MediaTree.albumTrackId(node.albumId, it.id), it.title, it.displayArtist ?: it.artist, coverUri(it.coverArt)))
                }
                items
            }
            is MediaNode.Artist -> artistDetail(node.artistId).albums.map { browsableAlbum(it) }.toMutableList()
            is MediaNode.Playlist -> playlistTrackItems(node.playlistId)
            else -> mutableListOf()
        }
    }

    // Recently played + random albums, mirroring HomeScreen's album rows. Artists have their own
    // top-level category, so they are not duplicated into Home.
    private suspend fun homeChildren(): MutableList<MediaItem> {
        val recent = recentAlbums()
        val seen = recent.map { it.id }.toMutableSet()
        val items = recent.map { browsableAlbum(it) }.toMutableList()
        randomAlbums().forEach { if (seen.add(it.id)) items.add(browsableAlbum(it)) }
        return items
    }

    private suspend fun playlistChildren(): MutableList<MediaItem> {
        val local = playlists.playlists.first()
        val items = local.map {
            browsable(MediaTree.playlistId(it.id), it.name, "${it.tracks.size} tracks", null)
        }.toMutableList()
        if (auth.isAuthenticated) {
            val matched = local.mapNotNull { it.serverId }.toSet()
            runCatching { api.getPlaylists() }.getOrDefault(emptyList())
                .filter { it.id !in matched }
                .forEach { items.add(browsable(MediaTree.playlistId(it.id), it.name, "${it.songCount} tracks", coverUri(it.coverArt))) }
        }
        return items
    }

    private suspend fun playlistTrackItems(playlistId: String): MutableList<MediaItem> {
        val localTracks = playlists.playlists.first().find { it.id == playlistId }?.tracks
        val tracks = localTracks ?: if (auth.isAuthenticated) api.getPlaylistTracks(playlistId).tracks else emptyList()
        val items = mutableListOf<MediaItem>()
        // A "Shuffle" entry at the top lets the car shuffle the whole playlist in one tap.
        if (tracks.isNotEmpty()) {
            items.add(playable(MediaTree.playlistShuffleId(playlistId), "Shuffle", "Shuffle this playlist", null))
        }
        tracks.forEach {
            items.add(playable(MediaTree.playlistTrackId(playlistId, it.id), it.title, it.displayArtist ?: it.artist, coverUri(it.coverArt)))
        }
        return items
    }

    // A–Z (+ "#") letter buckets for the Music browse node. No network — renders instantly.
    private fun musicLetters(): MutableList<MediaItem> {
        val buckets = ('A'..'Z').map { it.toString() } + "#"
        return buckets.map { browsable(MediaTree.musicLetterId(it), it, null, null) }.toMutableList()
    }

    private fun letterBucket(name: String): String {
        val c = name.trim().firstOrNull()?.uppercaseChar() ?: '#'
        return if (c in 'A'..'Z') c.toString() else "#"
    }

    // Albums are fetched once and reused across letter taps so a poor connection only pays the
    // cost a single time (and never blocks just opening the Music node).
    @Volatile private var albumCache: List<Album>? = null
    private suspend fun allAlbumsCached(): List<Album> = albumCache ?: albums().also { albumCache = it }

    private suspend fun searchItems(query: String): MutableList<MediaItem> {
        val results = if (auth.isAuthenticated) api.search(query) else localLibrary.search(query)
        val items = results.songs.map {
            // Encode the album context so tapping a result plays the album from that track.
            playable(MediaTree.albumTrackId(it.albumId, it.id), it.title, it.displayArtist ?: it.artist, coverUri(it.coverArt))
        }.toMutableList()
        results.albums.forEach { items.add(browsableAlbum(it)) }
        return items
    }

    // ── Data source (server when signed in, otherwise local library) ─────────────

    private suspend fun albums(): List<Album> = if (auth.isAuthenticated) api.getAlbums() else localLibrary.getAlbums()
    private suspend fun artists(): List<Artist> = if (auth.isAuthenticated) api.getArtists() else localLibrary.getArtists()
    private suspend fun recentAlbums(): List<Album> = if (auth.isAuthenticated) api.getRecentAlbums(12) else localLibrary.getRecentAlbums(12)
    private suspend fun randomAlbums(): List<Album> = if (auth.isAuthenticated) api.getRandomAlbums(12) else localLibrary.getRandomAlbums(12)
    private suspend fun albumDetail(id: String): Album = if (auth.isAuthenticated) api.getAlbumDetail(id) else localLibrary.getAlbumDetail(id)
    private suspend fun artistDetail(id: String) = if (auth.isAuthenticated) api.getArtistDetail(id) else localLibrary.getArtistDetail(id)

    // ── MediaItem builders ───────────────────────────────────────────────────────

    private fun browsableAlbum(album: Album): MediaItem =
        browsable(MediaTree.albumId(album.id), album.name, album.artist, coverUri(album.coverArt))

    private fun browsableArtist(artist: Artist): MediaItem =
        browsable(MediaTree.artistId(artist.id), artist.name, "${artist.albumCount} albums", coverUri(artist.coverArt))

    private fun browsable(mediaId: String, title: String, subtitle: String?, iconUri: Uri?): MediaItem =
        MediaItem(description(mediaId, title, subtitle, iconUri), MediaItem.FLAG_BROWSABLE)

    private fun playable(mediaId: String, title: String, subtitle: String?, iconUri: Uri?): MediaItem =
        MediaItem(description(mediaId, title, subtitle, iconUri), MediaItem.FLAG_PLAYABLE)

    private fun description(mediaId: String, title: String, subtitle: String?, iconUri: Uri?): MediaDescriptionCompat =
        MediaDescriptionCompat.Builder()
            .setMediaId(mediaId)
            .setTitle(title)
            .setSubtitle(subtitle)
            .apply { iconUri?.let { setIconUri(it) } }
            .build()

    // Server cover art is a Subsonic cover id -> authenticated URL; local art is already a URI.
    private fun coverUri(coverArt: String?): Uri? {
        if (coverArt.isNullOrBlank()) return null
        return when {
            coverArt.contains("://") -> Uri.parse(coverArt)
            auth.isAuthenticated -> Uri.parse(auth.coverArtUrl(coverArt, 256))
            else -> null
        }
    }

    private companion object {
        const val CONTENT_STYLE_SUPPORTED = "android.media.browse.CONTENT_STYLE_SUPPORTED"
        const val SEARCH_SUPPORTED = "android.media.browse.SEARCH_SUPPORTED"
        const val CONTENT_STYLE_BROWSABLE_HINT = "android.media.browse.CONTENT_STYLE_BROWSABLE_HINT"
        const val CONTENT_STYLE_PLAYABLE_HINT = "android.media.browse.CONTENT_STYLE_PLAYABLE_HINT"
        const val CONTENT_STYLE_LIST = 1
        const val CONTENT_STYLE_GRID = 2

        // System media browsers permitted to bind and enumerate the library.
        val ALLOWED_BROWSER_PACKAGES = setOf(
            "com.google.android.projection.gearhead", // Android Auto
            "com.google.android.carassistant",        // Android Automotive / Assistant Driving Mode
            "com.google.android.googlequicksearchbox", // Google Assistant
            "com.google.android.wearable.app",        // Wear OS companion
            "com.android.bluetooth",                  // Car/headunit Bluetooth media browsing
            "com.android.systemui",
        )
    }
}
