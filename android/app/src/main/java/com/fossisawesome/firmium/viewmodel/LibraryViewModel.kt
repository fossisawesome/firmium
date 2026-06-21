package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.local.LocalLibraryRepository
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Artist
import com.fossisawesome.firmium.data.model.ArtistDetail
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

// Lightweight artist entry derived from recent albums — id, display name, cover art.
data class RecentArtist(val id: String, val name: String, val coverArt: String?)

// Home screen data — equivalent to getRecentAlbums / getRandomAlbums in src/lib/api.ts (desktop).
data class HomeState(
    val recentAlbums: List<Album> = emptyList(),
    val recentArtists: List<RecentArtist> = emptyList(),
    val randomAlbums: List<Album> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

// Album list state.
data class AlbumListState(
    val albums: List<Album> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

// Artist list state.
data class ArtistListState(
    val artists: List<Artist> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

// Single album detail with tracks.
data class AlbumDetailState(
    val album: Album? = null,
    val isLoading: Boolean = false,
    val error: String? = null,
    // Ids of tracks that have a local (downloaded) copy. Empty when nothing is downloaded.
    val downloadedSongIds: Set<String> = emptySet(),
) {
    val allDownloaded: Boolean
        get() = album != null && album.tracks.isNotEmpty() && downloadedSongIds.size >= album.tracks.size
}

// Artist detail with albums and bio.
data class ArtistDetailState(
    val detail: ArtistDetail? = null,
    val isLoading: Boolean = false,
    val error: String? = null,
    // Similar artists the user actually has in their library ("You might also like").
    val recommendations: List<Artist> = emptyList(),
    // Top songs for the artist (server only), shown in the "Songs" preview section.
    val topSongs: List<com.fossisawesome.firmium.data.model.Song> = emptyList(),
)

class LibraryViewModel(app: Application) : AndroidViewModel(app) {

    private val api: ApiClient = getApplication<FirmiumApplication>().api
    private val auth: AuthManager = getApplication<FirmiumApplication>().auth
    private val localLibrary: LocalLibraryRepository = getApplication<FirmiumApplication>().localLibrary
    private val prefs = getApplication<FirmiumApplication>().prefs
    private val secureStorage = getApplication<FirmiumApplication>().secureStorage

    private fun albumKey(a: Album) = "${a.name.trim().lowercase()}|${a.artist.trim().lowercase()}"

    // Merges server and local album lists: server wins on duplicates (preserves server IDs for
    // scrobbling/lyrics), local-only albums are appended.
    private fun mergeAlbums(server: List<Album>, local: List<Album>): List<Album> {
        val serverKeys = server.map { albumKey(it) }.toHashSet()
        return server + local.filter { albumKey(it) !in serverKeys }
    }

    private fun mergeArtists(server: List<Artist>, local: List<Artist>): List<Artist> {
        val serverNames = server.map { it.name.trim().lowercase() }.toHashSet()
        return server + local.filter { it.name.trim().lowercase() !in serverNames }
    }

    private val _homeState = MutableStateFlow(HomeState())
    val homeState: StateFlow<HomeState> = _homeState.asStateFlow()

    private val _albumListState = MutableStateFlow(AlbumListState())
    val albumListState: StateFlow<AlbumListState> = _albumListState.asStateFlow()

    private val _artistListState = MutableStateFlow(ArtistListState())
    val artistListState: StateFlow<ArtistListState> = _artistListState.asStateFlow()

    private val _albumDetailState = MutableStateFlow(AlbumDetailState())
    val albumDetailState: StateFlow<AlbumDetailState> = _albumDetailState.asStateFlow()

    private val _artistDetailState = MutableStateFlow(ArtistDetailState())
    val artistDetailState: StateFlow<ArtistDetailState> = _artistDetailState.asStateFlow()

    // Resets all cached state and re-fetches the top-level lists from the now-current data
    // source (server vs. local library) — called after login/logout. Detail screens are
    // simply cleared; they re-fetch on their own when next navigated to.
    fun invalidateAll() {
        _albumDetailState.value = AlbumDetailState()
        _artistDetailState.value = ArtistDetailState()
        loadHome(force = true)
        loadAlbums(force = true)
        loadArtists(force = true)
    }

    fun loadHome(force: Boolean = false) {
        if (!force && _homeState.value.recentAlbums.isNotEmpty()) return
        if (_homeState.value.isLoading) return
        _homeState.value = HomeState(isLoading = true)
        viewModelScope.launch {
            try {
                val localDeferred = async { localLibrary.getAlbums() }
                val recentDeferred = async { if (auth.isAuthenticated) api.getRecentAlbums(12) else localLibrary.getRecentAlbums(12) }
                val randomDeferred = async { if (auth.isAuthenticated) api.getRandomAlbums(12) else localLibrary.getRandomAlbums(12) }
                val recent = recentDeferred.await()
                val random = randomDeferred.await()
                val localAlbums = if (auth.isAuthenticated) localDeferred.await() else emptyList()
                // Append local-only albums not represented in either server list.
                val serverKeys = (recent + random).map { albumKey(it) }.toHashSet()
                val localOnly = localAlbums.filter { albumKey(it) !in serverKeys }
                val mergedRecent = recent + localOnly.take(4)
                val mergedRandom = random + localOnly.drop(4).shuffled().take(4)
                // Derive unique artists from recent albums, preserving first-seen order.
                val artists = mutableListOf<RecentArtist>()
                val seen = mutableSetOf<String>()
                for (album in mergedRecent) {
                    if (album.artistId.isNotBlank() && seen.add(album.artistId)) {
                        artists.add(RecentArtist(album.artistId, album.artist, album.coverArt))
                    }
                }
                _homeState.value = HomeState(recentAlbums = mergedRecent, recentArtists = artists, randomAlbums = mergedRandom)
            } catch (e: Exception) {
                _homeState.value = HomeState(error = e.message)
            }
        }
    }

    fun loadAlbums(force: Boolean = false) {
        if (!force && _albumListState.value.albums.isNotEmpty()) return
        if (_albumListState.value.isLoading) return
        _albumListState.value = AlbumListState(isLoading = true)
        viewModelScope.launch {
            try {
                val albums = if (auth.isAuthenticated) {
                    val serverDeferred = async { api.getAlbums() }
                    val localDeferred = async { localLibrary.getAlbums() }
                    mergeAlbums(serverDeferred.await(), localDeferred.await())
                } else {
                    localLibrary.getAlbums()
                }
                _albumListState.value = AlbumListState(albums = albums)
            } catch (e: Exception) {
                _albumListState.value = AlbumListState(error = e.message)
            }
        }
    }

    fun loadArtists(force: Boolean = false) {
        if (!force && _artistListState.value.artists.isNotEmpty()) return
        if (_artistListState.value.isLoading) return
        _artistListState.value = ArtistListState(isLoading = true)
        viewModelScope.launch {
            try {
                val artists = if (auth.isAuthenticated) {
                    val serverDeferred = async { api.getArtists() }
                    val localDeferred = async { localLibrary.getArtists() }
                    mergeArtists(serverDeferred.await(), localDeferred.await())
                } else {
                    localLibrary.getArtists()
                }
                _artistListState.value = ArtistListState(artists = artists)
            } catch (e: Exception) {
                _artistListState.value = ArtistListState(error = e.message)
            }
        }
    }

    fun loadAlbumDetail(albumId: String) {
        if (_albumDetailState.value.album?.id == albumId) return
        _albumDetailState.value = AlbumDetailState(isLoading = true)
        viewModelScope.launch {
            try {
                val album = if (albumId.startsWith("local:")) localLibrary.getAlbumDetail(albumId) else api.getAlbumDetail(albumId)
                val downloaded = localLibrary.downloadedIds(album.tracks)
                _albumDetailState.value = AlbumDetailState(album = album, downloadedSongIds = downloaded)
            } catch (e: Exception) {
                _albumDetailState.value = AlbumDetailState(error = e.message)
            }
        }
    }

    // Recomputes downloaded marks for the current album (e.g. after a track or album download).
    fun refreshAlbumDownloaded() {
        val album = _albumDetailState.value.album ?: return
        viewModelScope.launch {
            localLibrary.invalidate()
            val downloaded = localLibrary.downloadedIds(album.tracks)
            _albumDetailState.value = _albumDetailState.value.copy(downloadedSongIds = downloaded)
        }
    }

    fun loadArtistDetail(artistId: String) {
        if (_artistDetailState.value.detail?.artist?.id == artistId) return
        _artistDetailState.value = ArtistDetailState(isLoading = true)
        viewModelScope.launch {
            try {
                val detail = if (artistId.startsWith("local:")) localLibrary.getArtistDetail(artistId) else api.getArtistDetail(artistId)
                _artistDetailState.value = ArtistDetailState(detail = detail)
                resolveRecommendations(artistId, detail.artist.name)
                loadArtistTopSongs(artistId, detail.artist.name)
            } catch (e: Exception) {
                _artistDetailState.value = ArtistDetailState(error = e.message)
            }
        }
    }

    // Loads the artist's top songs for the "Songs" preview (server only). Best-effort.
    private fun loadArtistTopSongs(artistId: String, artistName: String) {
        if (!auth.isAuthenticated || artistId.startsWith("local:")) return
        viewModelScope.launch {
            val songs = api.getTopSongs(artistName, 20)
            if (songs.isNotEmpty() && _artistDetailState.value.detail?.artist?.id == artistId) {
                _artistDetailState.value = _artistDetailState.value.copy(topSongs = songs)
            }
        }
    }

    // Resolves similar artists (server first, Last.fm fallback) cross-referenced
    // against the library so every suggestion is playable. Best-effort, non-blocking.
    private fun resolveRecommendations(artistId: String, artistName: String) {
        if (!auth.isAuthenticated || artistId.startsWith("local:")) return
        viewModelScope.launch {
            try {
                var names = api.getSimilarArtists(artistId)
                if (names.isEmpty() && prefs.lastfmEnabled.first()) {
                    val key = secureStorage.get("lastfm", "api_key") ?: ""
                    names = api.getLastfmSimilarArtists(artistName, key)
                }
                if (names.isEmpty()) return@launch
                val byName = api.getArtists().associateBy { it.name.lowercase() }
                val seen = HashSet<String>().apply { add(artistId) }
                val matched = names.mapNotNull { byName[it.lowercase()] }
                    .filter { seen.add(it.id) }
                    .take(12)
                if (matched.isNotEmpty() && _artistDetailState.value.detail?.artist?.id == artistId) {
                    _artistDetailState.value = _artistDetailState.value.copy(recommendations = matched)
                }
            } catch (_: Exception) { /* recommendations are best-effort */ }
        }
    }
}
