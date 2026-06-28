package com.fossisawesome.firmium.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query

@Dao
interface PodcastDao {
    @Insert
    suspend fun insertChannel(channel: PodcastChannelEntity)

    // Conflicting (channelId, guid) rows are ignored (already-seen episodes);
    // Room returns -1 for each ignored row, used to count actually-new episodes.
    @Insert(onConflict = OnConflictStrategy.IGNORE)
    suspend fun insertEpisodes(episodes: List<PodcastEpisodeEntity>): List<Long>

    @Query("SELECT * FROM podcast_channels ORDER BY addedAt DESC")
    suspend fun getChannels(): List<PodcastChannelEntity>

    @Query("SELECT * FROM podcast_episodes WHERE channelId = :channelId ORDER BY publishedAt DESC")
    suspend fun getEpisodes(channelId: String): List<PodcastEpisodeEntity>

    @Query("DELETE FROM podcast_episodes WHERE channelId = :channelId")
    suspend fun deleteEpisodesForChannel(channelId: String)

    @Query("DELETE FROM podcast_channels WHERE id = :channelId")
    suspend fun deleteChannel(channelId: String)

    @Query("UPDATE podcast_episodes SET positionMs = :positionMs WHERE id = :episodeId")
    suspend fun updatePosition(episodeId: String, positionMs: Long)
}
