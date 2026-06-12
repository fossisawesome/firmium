package com.fossisawesome.firmium.viewmodel

import com.fossisawesome.firmium.data.api.ApiClient
import com.fossisawesome.firmium.data.model.Song
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.mockito.kotlin.any
import org.mockito.kotlin.doReturn
import org.mockito.kotlin.mock
import org.mockito.kotlin.stub
import org.mockito.kotlin.times
import org.mockito.kotlin.verify

private fun song(id: String) = Song(
    id = id, title = id, artist = "Artist", displayArtist = null, album = "Album",
    albumId = "album-$id", artistId = "artist-1", duration = 180, track = null, year = null,
    genre = null, genres = emptyList(), coverArt = null, size = null, bitRate = null,
    samplingRate = null, bitDepth = null, suffix = null,
    replayGainTrack = null, replayGainAlbum = null, bpm = null,
)

@OptIn(ExperimentalCoroutinesApi::class)
class LyricsControllerTest {

    @Test
    fun fetchForTrack_populatesSyncedLines() = runTest {
        val api = mock<ApiClient>()
        val lines = listOf(ApiClient.LyricLine(0L, "line one"), ApiClient.LyricLine(1000L, "line two"))
        api.stub { onBlocking { getLyrics(any(), any(), any(), any(), any(), any()) } doReturn ApiClient.LyricsResult(lines, true) }

        val controller = LyricsController(this, api)
        controller.fetchForTrack(song("s1"))
        advanceUntilIdle()

        val state = controller.state.value
        assertFalse(state.isLoading)
        assertEquals(lines, state.lines)
        assertTrue(state.synced)
        assertEquals("s1", state.trackId)
    }

    @Test
    fun fetchForTrack_skipsRefetchWhenAlreadyLoaded() = runTest {
        val api = mock<ApiClient>()
        val lines = listOf(ApiClient.LyricLine(0L, "line one"))
        api.stub { onBlocking { getLyrics(any(), any(), any(), any(), any(), any()) } doReturn ApiClient.LyricsResult(lines, true) }

        val controller = LyricsController(this, api)
        controller.fetchForTrack(song("s1"))
        advanceUntilIdle()
        controller.fetchForTrack(song("s1"))
        advanceUntilIdle()

        verify(api, times(1)).getLyrics(any(), any(), any(), any(), any(), any())
    }

    @Test
    fun fetchForTrack_handlesNoLyricsFound() = runTest {
        val api = mock<ApiClient>()
        api.stub { onBlocking { getLyrics(any(), any(), any(), any(), any(), any()) } doReturn null }

        val controller = LyricsController(this, api)
        controller.fetchForTrack(song("s1"))
        advanceUntilIdle()

        val state = controller.state.value
        assertFalse(state.isLoading)
        assertTrue(state.lines.isEmpty())
    }

    @Test
    fun syncToPosition_advancesActiveLineAsPositionPasses() = runTest {
        val api = mock<ApiClient>()
        val lines = listOf(
            ApiClient.LyricLine(0L, "line 0"),
            ApiClient.LyricLine(1000L, "line 1"),
            ApiClient.LyricLine(2000L, "line 2"),
        )
        api.stub { onBlocking { getLyrics(any(), any(), any(), any(), any(), any()) } doReturn ApiClient.LyricsResult(lines, true) }

        val controller = LyricsController(this, api)
        controller.fetchForTrack(song("s1"))
        advanceUntilIdle()

        controller.syncToPosition(0.5)
        assertEquals(0, controller.state.value.activeLine)

        controller.syncToPosition(1.5)
        assertEquals(1, controller.state.value.activeLine)

        controller.syncToPosition(2.5)
        assertEquals(2, controller.state.value.activeLine)
    }
}
