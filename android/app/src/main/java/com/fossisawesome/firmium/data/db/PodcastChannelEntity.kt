package com.fossisawesome.firmium.data.db

import androidx.room.Entity
import androidx.room.PrimaryKey

// Client-side podcast subscription — Navidrome implements no server-side
// podcast endpoints (github.com/navidrome/navidrome/issues/793), so Firmium
// fetches/parses RSS feeds itself. Local only, no cross-device sync.
@Entity(tableName = "podcast_channels")
data class PodcastChannelEntity(
    @PrimaryKey val id: String,
    val feedUrl: String,
    val title: String,
    val description: String?,
    val imageUrl: String?,
    val addedAt: Long,
)
