package com.fossisawesome.firmium.audio

import com.fossisawesome.firmium.data.podcast.isPodcastEpisodeId
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PlaybackControllerPodcastTest {
    @Test
    fun `podcast ids are recognized and subsonic ids are not`() {
        assertTrue("podcast:ep1".isPodcastEpisodeId())
        assertFalse("a1b2c3-subsonic-song-id".isPodcastEpisodeId())
        assertFalse("local:content-uri".isPodcastEpisodeId())
    }
}
