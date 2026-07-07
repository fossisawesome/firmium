package com.fossisawesome.firmium.wear

// Wire protocol shared with the phone app's copy of this object (the two modules don't share
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

    // DataClient path for the active-account credentials pushed phone -> watch.
    const val AUTH_PATH = "/firmium/auth"

    // DataMap keys on AUTH_PATH. When KEY_HAS_ACCOUNT is false, the other three are absent
    // and the watch should clear any stored credentials.
    const val KEY_HAS_ACCOUNT = "has_account"
    const val KEY_SERVER_URL = "server_url"
    const val KEY_USERNAME = "username"
    const val KEY_PASSWORD = "password"

    // DataClient path for playback/appearance settings pushed phone -> watch.
    const val SETTINGS_PATH = "/firmium/settings"

    // DataMap keys on SETTINGS_PATH.
    const val KEY_CROSSFADE_ENABLED = "crossfade_enabled"
    const val KEY_CROSSFADE_DURATION = "crossfade_duration_ms"
    const val KEY_CROSSFADE_CURVE = "crossfade_curve"
    const val KEY_GAPLESS_ENABLED = "gapless_enabled"
    const val KEY_REPLAY_GAIN_ENABLED = "replay_gain_enabled"
    const val KEY_DOWNLOAD_FORMAT = "download_format"
    // Theme colors as "#RRGGBB" hex strings — resolved on the phone from the active theme id
    // (built-in or user-imported), so the watch never needs its own theme catalog.
    const val KEY_THEME_BG = "theme_bg"
    const val KEY_THEME_SURFACE = "theme_surface"
    const val KEY_THEME_SURFACE2 = "theme_surface2"
    const val KEY_THEME_TEXT = "theme_text"
    const val KEY_THEME_MUTED = "theme_muted"
    const val KEY_THEME_ACCENT = "theme_accent"
    const val KEY_THEME_ERROR = "theme_error"
    const val KEY_THEME_IS_DARK = "theme_is_dark"
}
