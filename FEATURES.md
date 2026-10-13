# Firmium Features

The standout features of Firmium on **desktop** (Linux/Windows/macOS) and **Android**. Routine basics (browse, search, seek, volume, playlist editing) are left out on purpose.

Legend: ✅ supported — ❌ not available — (blank) = same as ✅

---

## Playback

| Feature | Desktop | Android |
|---|:---:|:---:|
| Crossfade with configurable overlap and curve shape | ✅ | ✅ |
| Gapless playback | ✅ | ✅ |
| ReplayGain volume normalization | ✅ | ✅ |
| Bit-perfect audio mode — matches output rate to each track's native sample rate | ✅ | ❌ |
| Equalizer — 10-band graphic and parametric modes, saveable profiles, per-device assignment, `.toml` import | ✅ | ✅ |
| Cross-device play queue sync — resume exactly where you left off on any device | ✅ | ✅ |

---

## Discovery

| Feature | Desktop | Android |
|---|:---:|:---:|
| Synced lyrics with word-by-word karaoke fill | ✅ | ✅ |
| Similar tracks panel, works even on servers without native similarity support | ✅ | ✅ |
| Start Radio — build a queue seeded from any track, album, or artist | ✅ | ✅ |
| Mix — shuffled queue by energy level (Chill / Mid / High BPM) and genre | ✅ | ✅ |
| Continue playing after the queue ends with similar tracks | ✅ | ✅ |
| Community average ratings alongside your own, with a minimum-rating search filter | ✅ | ✅ |
| Podcasts — subscribe by RSS, resume episodes where you left off | ✅ | ✅ |

---

## Offline & Library

| Feature | Desktop | Android |
|---|:---:|:---:|
| Download tracks or albums, with a choice of original or transcoded format | ✅ | ✅ |
| Offline local library — plays downloaded music without a server connection | ✅ | ✅ |
| Drag-and-drop audio files or folders to import them | ✅ | ❌ |
| Local play history with CSV / JSON export | ✅ | ✅ |
| ListenBrainz scrobbling | ✅ | ✅ |

---

## Themes & Appearance

| Feature | Desktop | Android |
|---|:---:|:---:|
| 18+ built-in color themes plus custom themes via TOML | ✅ | ✅ |
| Spotify-style UI layout, independent of color theme | ❌ | ✅ |

---

## Android Ecosystem

- **Android Auto** — browse your full library, voice search, A–Z index, and "Up Next" on the car display
- **Wear OS** — remote control from the wrist, or standalone playback with offline downloads directly on the watch
- **Android TV** — big-screen library browsing, Now Playing with queue, lyrics, and similar tracks

---

## Desktop Extras

- **Command line control** — drive a running instance from the terminal (`firmium play-pause`, `next`, `volume`, `status`, etc.) for scripts and keyboard shortcuts
- **System media controls** — media keys and the OS now-playing widget (MPRIS, Windows media overlay, macOS Now Playing)

---

## Termium (Terminal Client)

A lightweight keyboard-driven TUI client that shares your login and library with the desktop app.

- **Browse, search, and play** your library entirely from the terminal
- **Resume server queue** — continue the queue you left on another device
- **Customizable keybindings** — vim-style by default
