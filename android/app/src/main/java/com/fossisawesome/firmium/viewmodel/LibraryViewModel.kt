package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Artist
import com.fossisawesome.firmium.data.model.ArtistDetail
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

// Lightweight artist entry derived from recent albums — id, display name, cover art.
data class RecentArtist(val id: String, val name: String, val coverArt: String?)

// Home screen data — mirrors getRecentAlbums / getRandomAlbums from api.js.
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
)

// Artist detail with albums and bio.
data class ArtistDetailState(
    val detail: ArtistDetail? = null,
    val isLoading: Boolean = false,
    val error: String? = null,
)

class LibraryViewModel(app: Application) : AndroidViewModel(app) {

    private val api: ApiClient = getApplication<FirmiumApplication>().api

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

    fun loadHome() {
        if (_homeState.value.recentAlbums.isNotEmpty()) return
        if (_homeState.value.isLoading) return
        _homeState.value = HomeState(isLoading = true)
        viewModelScope.launch {
            try {
                // Fetch recent and random albums concurrently — they are independent requests.
                val recentDeferred = async { api.getRecentAlbums(12) }
                val randomDeferred = async { api.getRandomAlbums(12) }
                val recent = recentDeferred.await()
                val random = randomDeferred.await()
                // Derive unique artists from recent albums, preserving first-seen order.
                val artists = mutableListOf<RecentArtist>()
                val seen = mutableSetOf<String>()
                for (album in recent) {
                    if (album.artistId.isNotBlank() && seen.add(album.artistId)) {
                        artists.add(RecentArtist(album.artistId, album.artist, album.coverArt))
                    }
                }
                _homeState.value = HomeState(recentAlbums = recent, recentArtists = artists, randomAlbums = random)
            } catch (e: Exception) {
                _homeState.value = HomeState(error = e.message)
            }
        }
    }

    fun loadAlbums() {
        if (_albumListState.value.albums.isNotEmpty()) return
        if (_albumListState.value.isLoading) return
        _albumListState.value = AlbumListState(isLoading = true)
        viewModelScope.launch {
            try {
                _albumListState.value = AlbumListState(albums = api.getAlbums())
            } catch (e: Exception) {
                _albumListState.value = AlbumListState(error = e.message)
            }
        }
    }

    fun loadArtists() {
        if (_artistListState.value.artists.isNotEmpty()) return
        if (_artistListState.value.isLoading) return
        _artistListState.value = ArtistListState(isLoading = true)
        viewModelScope.launch {
            try {
                _artistListState.value = ArtistListState(artists = api.getArtists())
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
                _albumDetailState.value = AlbumDetailState(album = api.getAlbumDetail(albumId))
            } catch (e: Exception) {
                _albumDetailState.value = AlbumDetailState(error = e.message)
            }
        }
    }

    fun loadArtistDetail(artistId: String) {
        if (_artistDetailState.value.detail?.artist?.id == artistId) return
        _artistDetailState.value = ArtistDetailState(isLoading = true)
        viewModelScope.launch {
            try {
                _artistDetailState.value = ArtistDetailState(detail = api.getArtistDetail(artistId))
            } catch (e: Exception) {
                _artistDetailState.value = ArtistDetailState(error = e.message)
            }
        }
    }
}
