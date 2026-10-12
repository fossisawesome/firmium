package com.fossisawesome.firmium.data.podcast

import com.fossisawesome.firmium.data.db.PodcastChannelEntity
import com.fossisawesome.firmium.data.db.PodcastEpisodeEntity
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PodcastSongAdapterTest {

    private val channel = PodcastChannelEntity(
        id = "chan1", feedUrl = "https://example.com/feed.xml", title = "My Show",
        description = "desc", imageUrl = "https://example.com/art.jpg", addedAt = 0L,
    )
    private val episode = PodcastEpisodeEntity(
        id = "ep1", channelId = "chan1", guid = "guid1", title = "Episode One",
        description = null, audioUrl = "https://example.com/ep1.mp3",
        durationSeconds = 1800L, publishedAt = 0L, positionMs = 5000L,
    )

    @Test
    fun `toSong prefixes id and pulls channel metadata`() {
        val song = episode.toSong(channel)
        assertEquals("podcast:ep1", song.id)
        assertEquals("Episode One", song.title)
        assertEquals("My Show", song.artist)
        assertEquals("My Show", song.album)
        assertEquals(1800, song.duration)
        assertEquals("https://example.com/art.jpg", song.coverArt)
    }

    @Test
    fun `isPodcastEpisodeId and podcastEpisodeId round-trip`() {
        val song = episode.toSong(channel)
        assertTrue(song.id.isPodcastEpisodeId())
        assertEquals("ep1", song.id.podcastEpisodeId())
        assertTrue(!"subsonic-track-id".isPodcastEpisodeId())
    }
}
