package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.api.AuthManager
import com.fossisawesome.firmium.data.api.SessionExpiredException
import com.fossisawesome.firmium.data.toUserError
import com.fossisawesome.firmium.data.local.LocalLibraryRepository
import com.fossisawesome.firmium.data.model.Album
import com.fossisawesome.firmium.data.model.Song
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class SearchState(
    val query: String = "",
    val songs: List<Song> = emptyList(),
    val albums: List<Album> = emptyList(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

class SearchViewModel(app: Application) : AndroidViewModel(app) {

    private val api: ApiClient = getApplication<FirmiumApplication>().api
    private val auth: AuthManager = getApplication<FirmiumApplication>().auth
    private val localLibrary: LocalLibraryRepository = getApplication<FirmiumApplication>().localLibrary
    private val _state = MutableStateFlow(SearchState())
    val state: StateFlow<SearchState> = _state.asStateFlow()

    private var searchJob: Job? = null

    // Debounces searches — waits 300ms after the last keystroke before firing.
    fun onQueryChanged(query: String) {
        _state.value = _state.value.copy(query = query)
        searchJob?.cancel()
        if (query.isBlank()) {
            _state.value = SearchState()
            return
        }
        searchJob = viewModelScope.launch {
            delay(300)
            _state.value = _state.value.copy(isLoading = true, error = null)
            try {
                val results = if (auth.isAuthenticated) api.search(query) else localLibrary.search(query)
                _state.value = _state.value.copy(
                    songs = results.songs,
                    albums = results.albums,
                    isLoading = false,
                )
            } catch (e: Exception) {
                if (e is kotlinx.coroutines.CancellationException) throw e
                if (e is SessionExpiredException) {
                    _state.value = _state.value.copy(isLoading = false, error = null)
                    return@launch
                }
                _state.value = _state.value.copy(isLoading = false, error = e.toUserError().message)
            }
        }
    }

    fun clearSearch() {
        searchJob?.cancel()
        _state.value = SearchState()
    }
}
