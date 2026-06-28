package com.fossisawesome.firmium.data.podcast

import com.fossisawesome.firmium.data.db.PodcastChannelEntity
import com.fossisawesome.firmium.data.db.PodcastDao
import com.fossisawesome.firmium.data.db.PodcastEpisodeEntity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.UUID
import java.util.concurrent.TimeUnit

class PodcastRepository(private val dao: PodcastDao) {

    private val httpClient = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(15, TimeUnit.SECONDS)
        .build()

    private suspend fun fetchFeed(feedUrl: String): ParsedFeed = withContext(Dispatchers.IO) {
        val request = Request.Builder().url(feedUrl).build()
        httpClient.newCall(request).execute().use { response ->
            val body = response.body ?: throw IllegalStateException("empty feed response")
            PodcastFeedParser.parse(body.byteStream())
        }
    }

    suspend fun addChannel(feedUrl: String): Result<PodcastChannelEntity> = try {
        val parsed = fetchFeed(feedUrl)
        val channel = PodcastChannelEntity(
            id = UUID.randomUUID().toString(),
            feedUrl = feedUrl,
            title = parsed.title,
            description = parsed.description,
            imageUrl = parsed.imageUrl,
            addedAt = System.currentTimeMillis() / 1000,
        )
        dao.insertChannel(channel)
        dao.insertEpisodes(parsed.episodes.map { it.toEntity(channel.id) })
        Result.success(channel)
    } catch (e: Exception) {
        Result.failure(e)
    }

    suspend fun refreshChannel(channelId: String, feedUrl: String): Result<Int> = try {
        val parsed = fetchFeed(feedUrl)
        val ids = dao.insertEpisodes(parsed.episodes.map { it.toEntity(channelId) })
        Result.success(ids.count { it != -1L })
    } catch (e: Exception) {
        Result.failure(e)
    }

    suspend fun getChannels(): List<PodcastChannelEntity> = dao.getChannels()

    suspend fun getEpisodes(channelId: String): List<PodcastEpisodeEntity> = dao.getEpisodes(channelId)

    suspend fun unsubscribe(channelId: String) {
        dao.deleteEpisodesForChannel(channelId)
        dao.deleteChannel(channelId)
    }

    suspend fun updatePosition(episodeId: String, positionMs: Long) = dao.updatePosition(episodeId, positionMs)

    private fun ParsedEpisode.toEntity(channelId: String) = PodcastEpisodeEntity(
        id = UUID.randomUUID().toString(),
        channelId = channelId,
        guid = guid,
        title = title,
        description = description,
        audioUrl = audioUrl,
        durationSeconds = durationSeconds,
        publishedAt = publishedAt,
        positionMs = 0,
    )
}
