# Firmium Features

A complete list of what Firmium can do, for both **desktop** (Linux/Windows/macOS) and **Android**.

Legend: ✅ supported — ❌ not available — (blank) = same as ✅

---

## Playback

| Feature | Desktop | Android |
|---|:---:|:---:|
| Crossfade between tracks with configurable overlap (1–12 s) | ✅ | ✅ |
| Crossfade curve shape — linear or logarithmic (equal-power) fade | ✅ | ✅ |
| Gapless playback — next track preloads silently, zero gap | ✅ | ✅ |
| ReplayGain volume normalization (toggle on/off) | ✅ | ✅ |
| Shuffle | ✅ | ✅ |
| Repeat modes — tap to cycle off, repeat all (forever), then repeat once | ✅ | ✅ |
| Seek bar | ✅ | ✅ |
| Volume control | ✅ | ✅ |
| Bit-perfect audio mode — matches output rate to track's native sample rate (Off / Relaxed / Strict) | ✅ | ❌ |
| Track format display (e.g. "FLAC · 96 kHz · 24-bit · 1411 kbps") | ✅ | ✅ |
| Equalizer — graphic (10-band) and parametric modes, saveable profiles, per-device assignment | ✅ | ✅ |
| Import equalizer profiles from `.toml` (desktop: read-only `eq-profiles/` drop folder; Android: file picker) | ✅ | ✅ |
| Audio stats panel — BPM and ReplayGain track/album gain and peak on the now-playing screen | ✅ | ✅ |

---

## Library & Browsing

| Feature | Desktop | Android |
|---|:---:|:---:|
| Browse albums, artists, and genres | ✅ | ✅ |
| Home screen — recently played, newest, and random albums | ✅ | ✅ |
| Artist pages with photos and Last.fm biography | ✅ | ✅ |
| Play button on artist page — instantly plays that artist's top tracks | ❌ | ✅ |
| Album detail pages with full track list | ✅ | ✅ |
| Search across your whole library | ✅ | ✅ |
| Track ratings — 1-5 star rating on tracks, synced to your server | ✅ | ✅ |
| Community average rating — see how other listeners on your server rated a track, shown next to your own rating | ✅ | ✅ |
| Filter search results by minimum rating (yours or the server-wide average, whichever is higher) | ✅ | ✅ |
| Genre and decade filter chips on the album list | ✅ | ✅ |
| BPM range filter on track lists (album detail, playlists) | ✅ | ✅ |

---

## Playlists

| Feature | Desktop | Android |
|---|:---:|:---:|
| Create and delete playlists | ✅ | ✅ |
| Rename playlists | ✅ | ✅ |
| Add and remove tracks | ✅ | ✅ |
| Reorder tracks (move up / move down) | ✅ | ✅ |
| Shuffle play a playlist | ✅ | ✅ |
| Play all | ✅ | ✅ |
| Sync playlists to your server | ✅ | ✅ |
| Manually push a local playlist to the server on demand | ✅ | ✅ |
| Cloud badge showing which playlists are synced | ✅ | ✅ |
| Mosaic cover art for playlists (up to 4 track covers) | ✅ | ✅ |

---

## Queue

| Feature | Desktop | Android |
|---|:---:|:---:|
| Full playback queue visible during playback | ✅ | ✅ |
| Jump to any track in the queue | ✅ | ✅ |
| Cross-device play queue sync — resume exactly where you left off on any device | ✅ | ✅ |

---

## Lyrics

| Feature | Desktop | Android |
|---|:---:|:---:|
| Synced (LRC) lyrics with real-time line highlighting | ✅ | ✅ |
| Word-by-word karaoke fill animation | ✅ | ✅ |
| Unsynced (plain text) lyrics | ✅ | ✅ |
| Lyrics panel background tinted to the track's cover art color | ✅ | ✅ |
| Multiple sources: server lyrics → LRCLIB fallback | ✅ | ✅ |
| Toggle word-by-word animation on/off | ✅ | ✅ |
| Lyrics shown in place of the cover art on the now-playing screen (tap art to show, X to close) | ❌ | ✅ |

---

## Similar Tracks

| Feature | Desktop | Android |
|---|:---:|:---:|
| Similar tracks panel — discover music related to what's playing | ✅ | ✅ |
| Works even on servers without native similarity support (genre + artist fallback) | ✅ | ✅ |

---

## Smart Radio & Mixes

| Feature | Desktop | Android |
|---|:---:|:---:|
| Continue playing after the queue ends — adds similar tracks automatically (toggle, off by default) | ✅ | ✅ |
| Mix — generate a shuffled queue by energy level (Chill / Mid / High BPM) and optional genre | ✅ | ✅ |
| Start Radio — build a queue seeded from any track, album, or artist and play it instantly | ✅ | ✅ |
| "You might also like" on artist pages — similar artists you already have in your library | ✅ | ✅ |

---

## Podcasts

| Feature | Desktop | Android |
|---|:---:|:---:|
| Subscribe to a podcast by RSS feed URL | ✅ | ✅ |
| Browse subscribed channels and episode lists | ✅ | ✅ |
| Play episodes through the regular player | ✅ | ✅ |
| Resume playback from where you left off in an episode | ✅ | ✅ |
| Manually refresh a channel for new episodes | ✅ | ✅ |
| Unsubscribe | ✅ | ✅ |

---

## Audio Visualizer

| Feature | Desktop | Android |
|---|:---:|:---:|
| GPU-accelerated visualizer — Bars, Lines, and Scope modes | ✅ | ✅ |
| Bars mode — frequency spectrum bars with peak hold indicators and per-bar flash on transients | ✅ | ❌ |
| Lines mode — smooth filled waveform with configurable glow and mirror effect | ✅ | ❌ |
| Scope (oscilloscope) mode — circular waveform ring with particle field driven by audio energy | ✅ | ❌ |
| Post-processing effects: bloom glow, motion trails, echo/Milkdrop feedback, CRT film grain | ✅ | ❌ |
| Beat-reactive bloom and pulse — effects intensify on bass hits | ✅ | ❌ |
| Toggle visualizer on/off | ✅ | ✅ |
| Switch visualizer mode from the panel header | ✅ | ✅ |
| Visualizer colors follow the album artwork (desktop: toggle in Settings, falls back to theme colors; Android: always-on) | ✅ | ✅ |

---

## Downloads & Offline

| Feature | Desktop | Android |
|---|:---:|:---:|
| Download individual tracks or whole albums from your server | ✅ | ✅ |
| Download your entire library at once, with progress, from Settings | ❌ | ✅ |
| Choose download format (original file or a transcode target) | ✅ | ✅ |
| Offline local library — plays downloaded music without a server connection | ✅ | ✅ |
| Downloaded tracks marked in track lists; albums and playlists marked when every track is saved | ❌ | ✅ |
| Re-download a track even when a local copy already exists (server mode) | ❌ | ✅ |
| Drag-and-drop audio files or folders into the app to import them | ✅ | ❌ |
| Automatically plays a local copy when available instead of streaming | ✅ | — |

---

## Stats & Recap

| Feature | Desktop | Android |
|---|:---:|:---:|
| Local play history — every completed track recorded on-device, no server needed | ✅ | ✅ |
| Firmium Recap — full-screen, swipeable cards of your listening (top tracks, artists, albums, genre, time of day, day of week, biggest discovery, streak) | ✅ | ✅ |
| Recap time ranges — 7 days, 30 days, 3 months, 1 year, all time | ✅ | ✅ |
| Custom Recap date range | ✅ | ❌ |
| Weekly Recap that appears automatically once a week | ✅ | ✅ |
| Save a Recap card as an image to share | ✅ | ✅ |
| Export play history as CSV or JSON | ✅ | ✅ |

---

## Themes & Appearance

| Feature | Desktop | Android |
|---|:---:|:---:|
| 18+ built-in color themes | ✅ | ✅ |
| Custom user themes via TOML files | ✅ | ✅ |
| Import custom themes from a `.toml` file (desktop: `themes/` drop folder; Android: file picker) | ✅ | ✅ |
| Choose the interface font from a curated list of 11 fonts | ✅ | ✅ |
| Responsive sidebar (collapses to icons, then a bottom tab bar on narrow windows) | ✅ | — |

**Built-in themes include:** Firmium, Gruvbox, Tokyo Night, Dracula, Catppuccin Mocha / Macchiato / Frappé / Latte, Monokai Classic, Monokai Pro, Adwaita, Adwaita Dark, ayu, ayu Light, GitHub Dark, Nordfox, Synthwave '84, Svalbard.

---

## Android Auto

| Feature | Android |
|---|:---:|
| Browse your full library from the car display (Home, Albums, Artists, Playlists) | ✅ |
| A–Z alphabet index for fast album browsing | ✅ |
| Voice search | ✅ |
| Search button on the car display | ✅ |
| Steering wheel transport controls (play, pause, skip, seek) | ✅ |
| Shuffle a whole playlist in one tap | ✅ |
| Shuffle a whole album in one tap | ✅ |
| "Up Next" queue visible on the car display | ✅ |
| Notification tinted to the track's cover art color | ✅ |
| Shuffle and repeat toggles on the car display | ✅ |

---

## Wear OS

| Feature | Android |
|---|:---:|
| Companion watch app — remote-controls playback running on your phone | ✅ |
| Now-playing display on the watch (title, artist, cover art) | ✅ |
| Transport controls from the wrist (play/pause, next, previous) | ✅ |
| Volume control via the rotating crown/bezel or on-screen buttons | ✅ |
| Standalone playback — browse and play your library directly on the watch, no phone needed | ✅ |
| Browse artists, albums, and playlists on the watch | ✅ |
| Search your library from the watch | ✅ |
| Crossfade, gapless playback, ReplayGain, shuffle, and repeat on watch playback | ✅ |

---

## Android TV

| Feature | Android |
|---|:---:|
| Browse your library on the big screen (Home, Albums, Artists, Playlists) | ✅ |
| Album, artist, and playlist detail pages | ✅ |
| Search | ✅ |
| Full-screen Now Playing with cover art and transport controls | ✅ |
| Queue panel on the Now Playing screen | ✅ |
| Synced lyrics panel on the Now Playing screen (line highlighting, no word-fill animation) | ✅ |
| Similar Tracks panel on the Now Playing screen | ✅ |
| Smart Mix (energy + genre picker) | ✅ |
| Equalizer — enable/select profile, adjust the 10 graphic bands (parametric editing, `.toml` import are phone/desktop-only) | ✅ |
| Recap & listening stats | ✅ |
| Settings — theme, font, crossfade, gapless, ReplayGain, visualizer, logout (Last.fm setup, cache/reset actions are phone/desktop-only) | ✅ |

---

## Android-Only Features

- **Full-screen player** — portrait and landscape layouts with cover art, controls, and visualizer
- **Player "more" menu** — a grid sheet on the now-playing screen for volume, add to playlist, visualizer, track info, view artist, add to queue, equalizer, and download
- **Long-press the cover art** — pops up the 1-5 star rating with an animation
- **Lock screen and notification controls** — persistent playback notification with cover art, shuffle and repeat buttons

---

## Desktop-Only Features

- **Bit-perfect audio** — open the output stream at each track's exact native sample rate
- **Window decorations toggle** — show or hide the native title bar and window borders from Settings
- **Wipe cover-art cache** — clear cached cover art from Settings → Debug
- **Reset preferences** — restore all settings to their defaults from Settings → Debug
- **Command line control** — control a running instance from the terminal (`firmium play-pause`, `next`, `prev`, `volume`, `seek`, `status`, etc.), for scripts and keyboard shortcuts

---

## Account & Security

| Feature | Desktop | Android |
|---|:---:|:---:|
| Credentials stored in OS keyring — never saved as plaintext | ✅ | ✅ |
| Multi-server quick switcher — save and switch between multiple servers | ✅ | ✅ |
| Auto-login on startup | ✅ | ✅ |
| Scrobbling — reports plays to the server (Last.fm via Navidrome, etc.) | ✅ | ✅ |
| ListenBrainz scrobbling — submits completed tracks to ListenBrainz with your user token | ✅ | ✅ |
| Playback reporting — keeps server "Now Playing" status accurate | ✅ | ✅ |
| Warning shown when connecting over plain HTTP to a non-local server | ✅ | ✅ |
| In-app error notifications: plain-language messages when something goes wrong (server unreachable, login failed, item not found), shown as dismissable notifications instead of failing silently | ✅ | ✅ |

---

## Format Support

Firmium can play any audio format your server streams, including MP3, FLAC, OGG Vorbis, Opus, AAC, WAV, and more.
