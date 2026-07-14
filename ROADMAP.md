# Firmium Roadmap

Not yet implemented — under consideration. See [FEATURES.md](FEATURES.md) for what's already shipped.

## Catching up to other Subsonic clients

- **Casting** — Chromecast/DLNA/AirPlay support for streaming to TVs and speakers
- **Sleep timer** — stop playback automatically after a set time
- **Multi-server quick switcher** — save and switch between multiple servers
- **Scrobbling** — reports plays to the server (Last.fm via Navidrome, etc.)

## Audio/DSP

- **Listening mode DSP presets** — Night (dynamic range compression + volume leveling), Car (bass boost + speech clarity) — one-tap profiles on top of existing EQ engine
- **Custom DSP chain** — user-chainable filters (compressor, stereo width, high-pass) beyond fixed EQ bands
- **A/B loudness-matched compare** — flip between two tracks/masters at matched perceived loudness (uses existing ReplayGain calc)

## Power-user/automation

- **Local HTTP control API** — trigger play/pause/queue from scripts, Home Assistant, Stream Deck, etc.
- **Persisted smart playlists** — save a BPM+genre+rating filter combo as an auto-refreshing playlist

## Social/discovery

- **Listen Party mode** — sync playback across multiple logged-in devices/accounts in real time, piggybacking on existing cross-device queue-sync infra
- **Recap Blend** — combine two accounts' local play history into a shared "you both love" mix, using existing Recap/stats engine

## Library intelligence

- **Duplicate/near-duplicate detection** — audio fingerprint across library, flags same track in different quality/album, offers to keep best version
- **Auto-generated liner notes** — pull production credits, recording year, personnel from MusicBrainz for album detail page
- **Library health dashboard** — missing album art, low-bitrate outliers, duplicate artists (tag variants), orphaned tracks — one screen, actionable fixes

## Playback/UX

- **Instant crossfade preview** — audition crossfade curve/length live on two tracks before committing to setting
- **Per-genre/per-playlist default settings** — settings that follow context (e.g. jazz playlist auto-disables crossfade) instead of a global toggle
- **Gesture-based scrubbing on Android** — swipe on album art for seek, long-press for scrub-speed ramp

## Sync/multi-device

- **Handoff** — desktop → phone (or vice versa) detects same account nearby (mDNS/local network), offers one-tap "continue here" mid-track
- **Shared local cache pool** — desktop + Android on same LAN share downloaded files instead of each downloading a full copy

## Accessibility/niche

- **Full keyboard-driven mode on desktop** — vim-style modal navigation across whole app
- **Colorblind-safe visualizer palettes** — dedicated accessible palette set, distinct from general theming
