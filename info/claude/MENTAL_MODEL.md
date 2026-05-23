# Firmium Desktop — Mental Model

## What it is

A native desktop Subsonic client. You point it at a Navidrome (or any Subsonic-compatible) server, it logs in, and lets you browse/play your music library. It is a Tauri v2 app: a Rust binary wraps a local webview that renders vanilla HTML/JS/CSS — no React, no bundler.

---

## The three-layer stack

```
┌─────────────────────────────────────────────────────────────┐
│  Webview (Chromium)                                         │
│  src/index.html  src/app.js  src/audio-bridge.js           │
│  — UI rendering, Subsonic API calls, playback logic        │
├─────────────────────────────────────────────────────────────┤
│  Tauri IPC (invoke / window.__TAURI__.core.invoke)          │
│  — JS ↔ Rust message passing over a local bridge           │
├─────────────────────────────────────────────────────────────┤
│  Rust backend  src-tauri/src/main.rs + audio.rs            │
│  — OS keyring, auth token generation, native audio          │
└─────────────────────────────────────────────────────────────┘
```

The webview does almost everything except three things that must live in Rust: credential storage (OS keyring), auth token generation (MD5, must stay off the JS side), and audio decoding/playback (native rodio engine).

---

## Startup sequence

1. `DOMContentLoaded` fires → `Store.Audio.init()` creates an `AudioBridge` instance.
2. Saved `firmium_server` / `firmium_user` are read from `localStorage`.
3. If `firmium_save_pass === 'true'`, the password is fetched from the **OS keyring** via `Keyring.load(user)` → Tauri IPC → `get_password` Rust command → `keyring` crate → libsecret (Linux).
4. The login form pre-fills. User hits Connect → `connect` action fires.
5. On connect: `generate_auth_params` is called in Rust (produces MD5 token + random salt), then `getAlbumList2` is called to validate. On success, `showApp()` switches views.

---

## Authentication

Subsonic uses token-auth: each API request needs `u` (username), `t` (MD5 of password+salt), `s` (random salt), `v`, `c`, `f`. 

The MD5 is computed **in Rust** (`generate_auth_params`) because doing crypto in JS inside a webview is fine but keeping the password out of JS-visible strings is cleaner. `Store.Auth.getQueryParams()` calls this command on every API request and returns the ready-to-append param map.

Credentials at rest:
- `firmium_server`, `firmium_user` → `localStorage` (not sensitive)
- Password → OS keyring only, never written to localStorage

---

## API layer

All Subsonic calls go through `Api.fetch(action, params, signal)`:

```
Api.fetch()
  → SubsonicRouter.buildUrl()        builds the full URL with auth params
  → fetch() (browser)                HTTP GET to the Subsonic server
  → parse JSON, check status         detects OpenSubsonic extensions
  → return responseObj
```

The `SubsonicMapper` normalises the raw Subsonic response fields into clean internal objects (`mapAlbum`, `mapArtist`, `mapSong`). OpenSubsonic extensions (like `displayArtist`, `releaseTypes`) are preferred over legacy fields.

---

## State management (`Store`)

A module-pattern singleton — no framework, just IIFEs returning closures.

| Module | Owns |
|--------|------|
| `Store.Auth` | server URL, username, password (in-memory only while logged in) |
| `Store.ServerInfo` | OpenSubsonic extension list, detected once per server |
| `Store.UI` | current view name (`albums`/`artists`/`search`/`settings`), back-nav stack |
| `Store.Playback` | queue (array of tracks), queue index, volume, repeat flags, cover art LRU cache, in-flight AbortControllers |
| `Store.Audio` | the `AudioBridge` instance, position polling interval, seeking flag |

---

## Playback pipeline

```
User clicks track
  → playAt(idx)
      → SubsonicRouter.buildUrl('stream', { id })   get the stream URL
      → AudioBridge.play(streamUrl, trackId)         JS → IPC → Rust
          → AudioPlayer.play_stream()
              → stop_track() for any existing session with same trackId
              → insert session immediately (loading=true) to avoid race
              → spawn_blocking: HTTP GET via reqwest blocking client
              → wrap response in StreamingReader (Read+Seek over live HTTP)
              → rodio Decoder::try_from(BufReader<StreamingReader>)
              → session.sink.append(source); sink.play()
              → session.loading = false
      → AudioBridge polls every 750ms via get_playback_state / is_playback_finished
      → on state change → emit('statechange', ...)  → update play button
      → on finish → emit('finished') → advance queue or repeat
  → Api.scrobble(id, false) — "now playing" ping
  → bridge.setVolume(savedVolume) — apply saved volume to new sink immediately
  → on finish → Api.scrobble(id, true) — submission ping
```

---

## `StreamingReader` — the key design choice

Instead of downloading the full audio file before playing, Firmium keeps the HTTP connection open and feeds bytes to the rodio decoder on demand. This matters for Navidrome's "Now Playing" admin view: Navidrome considers a track "now playing" as long as the stream connection is alive.

The `StreamingReader` also buffers every byte it consumes. This enables seeking:
- **Forward seek**: drain bytes from the live HTTP connection up to the target.
- **Backward seek**: bytes are already in the buffer; seek within it.
- **Native seek fails** (MP3/OGG forward-only decoders): rebuild decoder from the full in-memory buffer (`Cursor::new(raw_bytes)`) and seek from the beginning.

---

## Cover art loading

Cover art is fetched lazily via `IntersectionObserver`. When an `.lazy-art` `<img>` scrolls into view, `loadImage()` is called:

1. Check the LRU blob URL cache (`Store.Playback._covers`, max 150 entries).
2. If not cached, fetch `getCoverArt?id=...` from the Subsonic server, create a blob URL, cache it.
3. In-flight deduplication: a `_pendingCovers` map holds the in-progress promise so two images with the same cover ID share one fetch.

Blob URLs are revoked when entries are evicted from the cache to avoid memory leaks.

---

## UI rendering

No virtual DOM. The UI is entirely DOM manipulation via `innerHTML` string templates in `DOM.createAlbumCard`, `DOM.createArtistCard`, `DOM.createTrackCard`. `DOM.safeText()` HTML-escapes all user-supplied strings to prevent XSS.

Click handling uses **event delegation**: one listener on `document.body` (or a container div) dispatches on `data-action` attributes. This avoids attaching per-element listeners and handles dynamically rendered rows correctly.

---

## Views

| View | What renders in `listPanel` |
|------|---------------------------|
| `albums` | Flat alphabetical list of all albums (up to 500) |
| `artists` | Flat alphabetical list of all artists |
| album detail | Tracklist header + track rows; triggers on album card click |
| artist detail | Artist header with Wikipedia bio + photo; albums grouped by type (Albums / EPs / Singles) |
| `search` | Search input, then results split into Songs and Albums sections |
| `settings` | Theme selector, window decorations toggle, Wikipedia toggle |

Navigation uses a simple stack (`Store.UI._navHistory`). Pressing Back pops and re-calls the previous loader function. Loading a top-level view clears the stack.

---

## Repeat / queue logic

- **Repeat One**: on `finished`, call `playAt(currentIdx)`.
- **Repeat All**: on `finished` at end of queue, call `playAt(0)`.
- **Play token**: every call to `playAt()` bumps `Store.Playback._playToken`. The stream URL is fetched async; before playing, the token is checked against the current value. A superseded play request (e.g. user clicked skip during buffering) is silently dropped.

---

## AbortControllers

Every view-loading call (album list, artist page, etc.) creates a fresh `AbortController` and stores it in `Store.Playback._abortCtrl`. Starting a new load aborts the previous one, cancelling both the in-flight `fetch()` calls and any lazy-cover loads that reference the old signal.

Search has its own controller in `Store.Playback._searchCtrl` so typing quickly doesn't stack up results.

---

## Settings persistence

| Setting | Storage |
|---------|---------|
| Server URL | localStorage (`firmium_server`) |
| Username | localStorage (`firmium_user`) |
| Password | OS keyring (libsecret / kwallet) |
| Save password flag | localStorage (`firmium_save_pass`) |
| Volume | localStorage (`firmium_volume`) |
| Theme | localStorage (`firmium_theme`) |
| Window decorations | localStorage (`firmium_decorations`) |
| Wikipedia enabled | localStorage (`firmium_wikipedia`) |

---

## Key files

| File | Role |
|------|------|
| `src/index.html` | Shell HTML, loads CSS and both scripts |
| `src/app.js` | All UI logic, Subsonic API, state, playback orchestration |
| `src/audio-bridge.js` | JS wrapper over Tauri IPC for audio; exposes `AudioBridge` class |
| `src/style.css` | All CSS including themes via `data-theme` on `<html>` |
| `src-tauri/src/main.rs` | Tauri command handlers: keyring, auth params, audio control, system info |
| `src-tauri/src/audio.rs` | `AudioPlayer` struct: rodio integration, `StreamingReader`, session management |
| `src-tauri/Cargo.toml` | Rust dependencies (rodio, reqwest, keyring, parking_lot, uuid…) |
| `src-tauri/tauri.conf.json` | App config: window size, identifier, bundle targets |

---

## Dependency highlights

| Crate | Purpose |
|-------|---------|
| `rodio 0.22` | Audio decoding and playback (wraps symphonia) |
| `reqwest` (blocking) | HTTP streaming of audio from Subsonic |
| `keyring 3` | OS credential store (libsecret on Linux) |
| `parking_lot` | Fast Mutex/RwLock for session state |
| `uuid` | Unique player session IDs |
| `md5` + `rand` | Subsonic token-auth (password + random salt → MD5 token) |
| `sysinfo` | CPU/distro info for settings page |
| `tauri 2.x` | App shell, IPC, window management |
