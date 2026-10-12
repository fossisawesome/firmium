package com.fossisawesome.firmium.data.podcast

import com.fossisawesome.firmium.data.db.PodcastChannelEntity
import com.fossisawesome.firmium.data.db.PodcastEpisodeEntity
import com.fossisawesome.firmium.data.model.Song

const val PODCAST_ID_PREFIX = "podcast:"

fun String.isPodcastEpisodeId(): Boolean = startsWith(PODCAST_ID_PREFIX)
fun String.podcastEpisodeId(): String = removePrefix(PODCAST_ID_PREFIX)

// Adapts a podcast episode into the shared Song model so it can flow through
// PlaybackController/PlayerState (mini-player, full-screen player) the same way
// Subsonic tracks do. The "podcast:" id prefix (same convention as the existing
// "local:" prefix for local-library tracks) marks it so PlaybackController can
// skip Subsonic-only side effects (scrobble, rating, play history).
fun PodcastEpisodeEntity.toSong(channel: PodcastChannelEntity): Song = Song(
    id = "$PODCAST_ID_PREFIX$id",
    title = title,
    artist = channel.title,
    displayArtist = null,
    album = channel.title,
    albumId = channel.id,
    artistId = channel.id,
    duration = (durationSeconds ?: 0L).toInt(),
    track = null,
    year = null,
    genre = null,
    genres = emptyList(),
    coverArt = channel.imageUrl,
    size = null,
    bitRate = null,
    samplingRate = null,
    bitDepth = null,
    suffix = null,
    replayGainTrack = null,
    replayGainAlbum = null,
    replayGainTrackPeak = null,
    replayGainAlbumPeak = null,
    bpm = null,
    userRating = null,
    averageRating = null,
    starred = false,
)
