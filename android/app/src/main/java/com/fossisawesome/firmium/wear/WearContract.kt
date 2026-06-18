package com.fossisawesome.firmium.wear

// Wire protocol shared with the wear module's copy of this object (the two modules don't share
// code, so the constants are duplicated and MUST stay in sync).
object WearContract {
    // MessageClient path for transport commands sent watch -> phone.
    const val CMD_PATH = "/firmium/cmd"

    // DataClient path for the now-playing snapshot pushed phone -> watch.
    const val NOW_PLAYING_PATH = "/firmium/now_playing"

    // Command payloads (UTF-8 message body on CMD_PATH).
    const val CMD_PLAY_PAUSE = "play_pause"
    const val CMD_NEXT = "next"
    const val CMD_PREV = "prev"
    const val CMD_SET_VOLUME_PREFIX = "set_volume:" // followed by a 0.0-1.0 float

    // DataMap keys on NOW_PLAYING_PATH.
    const val KEY_HAS_TRACK = "has_track"
    const val KEY_TITLE = "title"
    const val KEY_ARTIST = "artist"
    const val KEY_ALBUM = "album"
    const val KEY_IS_PLAYING = "is_playing"
    const val KEY_VOLUME = "volume"
    const val KEY_TRACK_ID = "track_id"
    const val KEY_ART = "art" // Asset: downscaled JPEG cover art
}
