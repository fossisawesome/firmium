package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.db.PodcastChannelEntity
import com.fossisawesome.firmium.data.db.PodcastEpisodeEntity
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class PodcastsViewModel(app: Application) : AndroidViewModel(app) {

    private val repo = getApplication<FirmiumApplication>().podcasts

    private val _channels = MutableStateFlow<List<PodcastChannelEntity>>(emptyList())
    val channels: StateFlow<List<PodcastChannelEntity>> = _channels.asStateFlow()

    private val _episodes = MutableStateFlow<List<PodcastEpisodeEntity>>(emptyList())
    val episodes: StateFlow<List<PodcastEpisodeEntity>> = _episodes.asStateFlow()

    private val _addError = MutableStateFlow<String?>(null)
    val addError: StateFlow<String?> = _addError.asStateFlow()

    fun loadChannels() {
        viewModelScope.launch { _channels.value = repo.getChannels() }
    }

    fun addChannel(feedUrl: String) {
        viewModelScope.launch {
            repo.addChannel(feedUrl).fold(
                onSuccess = {
                    _addError.value = null
                    loadChannels()
                },
                onFailure = { _addError.value = it.message ?: "Failed to add podcast" },
            )
        }
    }

    fun loadEpisodes(channelId: String) {
        viewModelScope.launch { _episodes.value = repo.getEpisodes(channelId) }
    }

    fun refreshChannel(channelId: String, feedUrl: String) {
        viewModelScope.launch {
            repo.refreshChannel(channelId, feedUrl)
            loadEpisodes(channelId)
        }
    }

    fun unsubscribe(channelId: String) {
        viewModelScope.launch {
            repo.unsubscribe(channelId)
            loadChannels()
        }
    }
}
