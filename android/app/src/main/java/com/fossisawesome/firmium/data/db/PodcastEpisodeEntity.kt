package com.fossisawesome.firmium.data.db

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "podcast_episodes",
    indices = [Index("channelId"), Index(value = ["channelId", "guid"], unique = true)],
)
data class PodcastEpisodeEntity(
    @PrimaryKey val id: String,
    val channelId: String,
    val guid: String,
    val title: String,
    val description: String?,
    val audioUrl: String,
    val durationSeconds: Long?,
    val publishedAt: Long?,
    val positionMs: Long = 0,
)
