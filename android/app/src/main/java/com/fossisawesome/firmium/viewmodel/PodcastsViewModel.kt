package com.fossisawesome.firmium.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.fossisawesome.firmium.FirmiumApplication
import com.fossisawesome.firmium.data.db.PodcastChannelEntity
import com.fossisawesome.firmium.data.db.PodcastEpisodeEntity
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

class PodcastsViewModel(app: Application) : AndroidViewModel(app) {

    private val repo = getApplication<FirmiumApplication>().podcasts
    private val audioPlayer = getApplication<FirmiumApplication>().audioPlayer

    private val _channels = MutableStateFlow<List<PodcastChannelEntity>>(emptyList())
    val channels: StateFlow<List<PodcastChannelEntity>> = _channels.asStateFlow()

    private val _episodes = MutableStateFlow<List<PodcastEpisodeEntity>>(emptyList())
    val episodes: StateFlow<List<PodcastEpisodeEntity>> = _episodes.asStateFlow()

    private val _addError = MutableStateFlow<String?>(null)
    val addError: StateFlow<String?> = _addError.asStateFlow()

    private val _playingEpisodeId = MutableStateFlow<String?>(null)
    val playingEpisodeId: StateFlow<String?> = _playingEpisodeId.asStateFlow()

    private var currentPlayerId: String? = null
    private var positionJob: Job? = null

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

    // Plays an episode through the shared AudioPlayer engine directly (bypassing
    // PlaybackController's Song-queue/scrobble logic — episodes aren't Subsonic
    // tracks). PlaybackController's listener ignores callbacks for player ids it
    // didn't start, so this can't interfere with regular music playback.
    fun playEpisode(episode: PodcastEpisodeEntity) {
        positionJob?.cancel()
        val playerId = audioPlayer.play(episode.audioUrl, episode.id)
        currentPlayerId = playerId
        _playingEpisodeId.value = episode.id
        if (episode.positionMs > 0) {
            audioPlayer.seek(playerId, episode.positionMs / 1000.0)
        }
        positionJob = viewModelScope.launch {
            while (isActive) {
                delay(1000)
                val pid = currentPlayerId ?: break
                val posMs = (audioPlayer.getPosition(pid) * 1000).toLong()
                repo.updatePosition(episode.id, posMs)
            }
        }
    }

    override fun onCleared() {
        super.onCleared()
        positionJob?.cancel()
    }
}
