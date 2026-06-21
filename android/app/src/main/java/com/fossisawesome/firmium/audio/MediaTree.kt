package com.fossisawesome.firmium.audio

// Identifiers for the Android Auto media-browser tree, and parsing them back into typed nodes.
//
// Browsable container ids use "<type>:<rawId>" where rawId may itself contain ':' (local-library
// ids look like "local:12345"), so parsing strips only the leading type prefix.
//
// Playable track ids use '|' as the delimiter ("track|album|<albumId>|<songId>"): Subsonic and
// local-library song ids never contain '|', so the container id is everything up to the final
// '|' and the song id is the last segment. This keeps the container id opaque even if it
// contains ':'.
object MediaTree {
    const val ROOT = "root"
    const val HOME = "home"
    const val MUSIC = "music"
    const val ARTISTS = "artists"
    const val PLAYLISTS = "playlists"

    fun albumId(albumId: String) = "album:$albumId"
    fun artistId(artistId: String) = "artist:$artistId"
    fun playlistId(playlistId: String) = "playlist:$playlistId"
    // A–Z index node under "Music"; bucket is a single letter A-Z or "#" for non-alphabetic.
    fun musicLetterId(bucket: String) = "music_letter:$bucket"
    // Special playable that shuffles a whole playlist.
    fun playlistShuffleId(playlistId: String) = "shuffle|playlist|$playlistId"
    // Special playable that shuffles a whole album.
    fun albumShuffleId(albumId: String) = "shuffle|album|$albumId"

    fun albumTrackId(albumId: String, songId: String) = "track|album|$albumId|$songId"
    fun playlistTrackId(playlistId: String, songId: String) = "track|playlist|$playlistId|$songId"

    fun parse(mediaId: String): MediaNode = when {
        mediaId == ROOT -> MediaNode.Root
        mediaId == HOME -> MediaNode.Home
        mediaId == MUSIC -> MediaNode.Music
        mediaId == ARTISTS -> MediaNode.Artists
        mediaId == PLAYLISTS -> MediaNode.Playlists
        mediaId.startsWith("music_letter:") -> MediaNode.MusicLetter(mediaId.removePrefix("music_letter:"))
        mediaId.startsWith("shuffle|playlist|") -> MediaNode.PlaylistShuffle(mediaId.removePrefix("shuffle|playlist|"))
        mediaId.startsWith("shuffle|album|") -> MediaNode.AlbumShuffle(mediaId.removePrefix("shuffle|album|"))
        mediaId.startsWith("track|album|") -> {
            val body = mediaId.removePrefix("track|album|")
            MediaNode.AlbumTrack(body.substringBeforeLast('|'), body.substringAfterLast('|'))
        }
        mediaId.startsWith("track|playlist|") -> {
            val body = mediaId.removePrefix("track|playlist|")
            MediaNode.PlaylistTrack(body.substringBeforeLast('|'), body.substringAfterLast('|'))
        }
        mediaId.startsWith("album:") -> MediaNode.Album(mediaId.removePrefix("album:"))
        mediaId.startsWith("artist:") -> MediaNode.Artist(mediaId.removePrefix("artist:"))
        mediaId.startsWith("playlist:") -> MediaNode.Playlist(mediaId.removePrefix("playlist:"))
        else -> MediaNode.Unknown(mediaId)
    }
}

sealed interface MediaNode {
    object Root : MediaNode
    object Home : MediaNode
    object Music : MediaNode
    object Artists : MediaNode
    object Playlists : MediaNode
    data class MusicLetter(val bucket: String) : MediaNode
    data class PlaylistShuffle(val playlistId: String) : MediaNode
    data class AlbumShuffle(val albumId: String) : MediaNode
    data class Album(val albumId: String) : MediaNode
    data class Artist(val artistId: String) : MediaNode
    data class Playlist(val playlistId: String) : MediaNode
    data class AlbumTrack(val albumId: String, val songId: String) : MediaNode
    data class PlaylistTrack(val playlistId: String, val songId: String) : MediaNode
    data class Unknown(val raw: String) : MediaNode
}
