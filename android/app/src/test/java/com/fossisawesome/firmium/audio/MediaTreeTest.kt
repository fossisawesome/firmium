package com.fossisawesome.firmium.audio

import org.junit.Assert.assertEquals
import org.junit.Test

class MediaTreeTest {

    @Test
    fun parse_categoryIds_returnCategoryNodes() {
        assertEquals(MediaNode.Root, MediaTree.parse(MediaTree.ROOT))
        assertEquals(MediaNode.Home, MediaTree.parse(MediaTree.HOME))
        assertEquals(MediaNode.Albums, MediaTree.parse(MediaTree.ALBUMS))
        assertEquals(MediaNode.Artists, MediaTree.parse(MediaTree.ARTISTS))
        assertEquals(MediaNode.Playlists, MediaTree.parse(MediaTree.PLAYLISTS))
    }

    @Test
    fun albumId_roundTrips() {
        assertEquals(MediaNode.Album("a1"), MediaTree.parse(MediaTree.albumId("a1")))
    }

    @Test
    fun artistId_roundTrips() {
        assertEquals(MediaNode.Artist("ar1"), MediaTree.parse(MediaTree.artistId("ar1")))
    }

    @Test
    fun playlistId_roundTrips() {
        assertEquals(MediaNode.Playlist("pl1"), MediaTree.parse(MediaTree.playlistId("pl1")))
    }

    @Test
    fun containerId_preservesRawIdContainingColon() {
        // Local-library ids look like "local:12345" — the colon must survive parsing.
        assertEquals(MediaNode.Album("local:12345"), MediaTree.parse(MediaTree.albumId("local:12345")))
    }

    @Test
    fun albumTrackId_roundTripsToAlbumIdAndSongId() {
        val id = MediaTree.albumTrackId("alb1", "song9")
        assertEquals(MediaNode.AlbumTrack("alb1", "song9"), MediaTree.parse(id))
    }

    @Test
    fun playlistTrackId_roundTripsToPlaylistIdAndSongId() {
        val id = MediaTree.playlistTrackId("pl1", "song9")
        assertEquals(MediaNode.PlaylistTrack("pl1", "song9"), MediaTree.parse(id))
    }

    @Test
    fun albumTrackId_preservesContainerIdContainingColon() {
        // The album id keeps everything up to the final delimiter; the song id is the last segment.
        val id = MediaTree.albumTrackId("local:album:7", "local:99")
        assertEquals(MediaNode.AlbumTrack("local:album:7", "local:99"), MediaTree.parse(id))
    }

    @Test
    fun parse_unknownId_returnsUnknown() {
        assertEquals(MediaNode.Unknown("garbage"), MediaTree.parse("garbage"))
    }
}
