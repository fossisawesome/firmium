# Navidrome API Reference

Reference for Navidrome's API surface as it relates to Firmium's integration. Covers the Subsonic/OpenSubsonic endpoints Firmium uses, authentication, browsing and search, media streaming, and Navidrome's native REST API (web UI only).

Navidrome targets **Subsonic API v1.16.1** with OpenSubsonic extensions. Some standard Subsonic features are intentionally omitted or work differently — see [Navidrome-specific Caveats](#navidrome-specific-caveats) below.

---

## API Layer Overview

Navidrome exposes three distinct API interfaces via a `chi.Router`, each mounted at a different path:

| Interface | Path | Used by |
|-----------|------|---------|
| Subsonic/OpenSubsonic | `/rest/*` | Firmium (desktop + Android), all third-party clients |
| Native REST | `/api/*` | Navidrome web UI only |
| Media streaming | embedded in Subsonic | All clients |

All three share global middleware: security headers, CORS, IP detection, and JWT verification. Dependency injection is handled at compile time via Google's `wire` library (`CreateSubsonicAPIRouter`, `CreateNativeAPIRouter`, `CreateServer`).

---

## Subsonic / OpenSubsonic API

### Base Path and Formats

- Base path: `/rest/`
- API version targeted: **1.16.1**
- OpenSubsonic extensions enabled by default
- Response format controlled by `f` parameter:
  - `f=json` — JSON wrapped in `subsonic-response` object (Firmium uses this)
  - `f=xml` — XML (default if omitted)
  - `f=jsonp` — JSONP with callback

### Authentication

Every request must include these query parameters:

| Parameter | Description |
|-----------|-------------|
| `u` | Username |
| `v` | API version (Firmium sends `1.16.1`) |
| `c` | Client name (Firmium sends `firmium`) |
| `f` | Response format (`json`) |

Plus one of the following credential methods (in priority order Navidrome evaluates them):

1. **Reverse proxy** — username extracted from `Remote-User` header; `u` parameter ignored when a trusted proxy is whitelisted
2. **Token/MD5** (standard) — `t` = `md5(password + salt)`, `s` = random salt. This is what Firmium uses — see `commands/auth.rs::generate_auth_params()`
3. **Password** — `p` parameter; supports hex-encoding with `enc:` prefix
4. **JWT** — `jwt` parameter validated against Navidrome session tokens

Firmium always uses **Token/MD5**. The MD5 token is generated on the Rust side; plaintext passwords never reach the frontend.

### Middleware Pipeline

Every Subsonic request passes through these layers in order:

1. **postFormToQueryParams** — converts POST form data to query params
2. **checkRequiredParameters** — validates `u`, `v`, `c` are present
3. **authenticate** — verifies credentials, injects user context
4. **getPlayer** — registers client as a player (by IP + User-Agent + cookie), stores player info in request context

Firmium triggers `firmium:session-expired` on HTTP 401 or Subsonic error codes 40/41 (see `commands/subsonic.rs::subsonic_request()`).

---

## Browsing and Navigation

Navidrome uses **ID3-based** browsing (tag-based, not folder-based). The hierarchy is:

```
Music Folders → Artist Indexes → Artist → Albums → Tracks
```

### Key Endpoints

#### `getSong`
Returns metadata for a single song by ID.

#### `getMusicFolders`
Returns the libraries accessible to the authenticated user. Each entry has `id` and `name`. Firmium passes `musicFolderId` to filter requests to a specific library.

#### `getArtists`
Returns all artists grouped alphabetically by initial letter. Filters to album artists by default. Includes `lastModified` timestamps for client-side cache invalidation (derived from scan time).

#### `getArtist`
Returns a single artist with all associated albums. Used by `ArtistDetail.svelte` via `api.ts`.

#### `getAlbum`
Returns album metadata and full track listing. Used by `AlbumDetail.svelte`.

#### `getAlbumList` / `getAlbumList2`
Returns curated album lists. Supported `type` values:

- `newest` — recently added
- `recent` — recently played
- `random` — random selection
- `alphabeticalByName` / `alphabeticalByArtist`
- `frequent` — most played
- `starred` — user-starred albums
- `highest` — highest rated
- `byGenre` — filtered by genre (requires `genre` param)
- `byYear` — filtered by year range (requires `fromYear`, `toYear`)

Firmium fetches albums paginated with `maxItems=500` (see `commands/subsonic.rs`).

#### `getGenres`
Returns all genres present in the library.

#### `getStarred` / `getStarred2`
Returns starred artists, albums, and songs. Navidrome retrieves these in parallel internally.

#### `getNowPlaying`
Returns tracks currently being played across all active sessions.

#### `getRandomSongs`
Returns a random set of songs. Supports filtering by genre, year range, music folder, and count.

#### `getSongsByGenre`
Returns songs filtered by a specific genre tag.

#### `getSimilarSongs` / `getSimilarSongs2`
Returns songs similar to a given artist or song. Requires Last.fm integration to be configured on the server; returns empty results otherwise.

#### `getTopSongs`
Returns top songs for an artist. Also requires Last.fm integration.

#### `getArtistInfo` / `getArtistInfo2`
Returns artist biography, MusicBrainz ID, and similar artists. Requires Last.fm and/or MusicBrainz integration on the server side.

#### `getAlbumInfo` / `getAlbumInfo2`
Returns album notes and MusicBrainz ID. Requires external integrations.

---

## Folder-Based Browsing (Simulated)

Navidrome is tag-based, not folder-based. The following endpoints exist for client compatibility but return **simulated** data, not the actual filesystem:

| Endpoint | Behavior |
|----------|----------|
| `getIndexes` | Returns artists as a simulated index |
| `getMusicDirectory` | Returns a fake directory tree built from tags |

Avoid relying on these for any logic that needs to reflect real file paths or directory structure. Prefer `getArtists` / `getAlbum` for all navigation.

---

## Search

Two search endpoints with different response shapes:

| Endpoint | Response type | Element types |
|----------|--------------|---------------|
| `search2` | `SearchResult2` | `Artist`, `Child` (legacy) |
| `search3` | `SearchResult3` | `ArtistID3`, `AlbumID3` (modern) |

Firmium uses `search3` via `commands/subsonic.rs::search()`, limited to 40 albums and 100 songs per query.

### Query processing (Navidrome internals)
1. Accents stripped, query lowercased
2. `model.QueryOptions` built with optional `library_id` filter
3. Artist, album, and song searches run in parallel via `errgroup`

### Response adaptation
Navidrome adjusts response fields based on detected client type:

- **Minimal clients** — only `Id`, `Title`, `IsDir`
- **Legacy clients** — OpenSubsonic extensions suppressed
- **Modern clients** — full extended metadata

Firmium is treated as a modern client and receives the full OpenSubsonic metadata.

---

## User Annotations

| Endpoint | Action |
|----------|--------|
| `star` | Star an artist, album, or song |
| `unstar` | Remove star |
| `setRating` | Set 1-5 star rating |
| `scrobble` | Record a play (submission + now-playing) |

Firmium calls `scrobble` via `commands/subsonic.rs::scrobble()`. This updates the Navidrome play count and submits to Last.fm if the user has configured that integration on the server side.

---

## Playlists

| Endpoint | Action |
|----------|--------|
| `getPlaylists` | List all playlists |
| `getPlaylist` | Get playlist with tracks |
| `createPlaylist` | Create or update a playlist |
| `updatePlaylist` | Modify name, comment, public flag, add/remove songs |
| `deletePlaylist` | Delete a playlist |

**Smart playlists** are read-only; Navidrome re-evaluates their contents automatically based on refresh interval. They include a `validUntil` field.

Firmium exposes playlist CRUD via `PlaylistsView.svelte` and `PlaylistDetail.svelte`, backed by `api.ts` wrappers.

---

## Bookmarks and Play Queue

| Endpoint | Action |
|----------|--------|
| `getBookmarks` | List all bookmarks (position saves) for the user |
| `createBookmark` | Save playback position for a song |
| `deleteBookmark` | Remove a bookmark |
| `getPlayQueue` | Retrieve the user's persisted play queue (cross-client sync) |
| `savePlayQueue` | Persist the current play queue and position |

These enable cross-client resume: a queue saved from one client can be picked up by another. Firmium does not currently implement these but the endpoints are available.

---

## Sharing

Sharing must be enabled in Navidrome's server config. When available:

| Endpoint | Action |
|----------|--------|
| `getShares` | List existing public share links |
| `createShare` | Create a share link for songs, albums, or playlists |
| `updateShare` | Update share description or expiry |
| `deleteShare` | Delete a share link |

Share links allow unauthenticated access to specific media using encoded tokens.

---

## Internet Radio

| Endpoint | Action |
|----------|--------|
| `getInternetRadioStations` | List configured radio stations |
| `createInternetRadioStation` | Add a station (admin) |
| `updateInternetRadioStation` | Edit a station (admin) |
| `deleteInternetRadioStation` | Remove a station (admin) |

---

## Media Streaming

### `stream`

Streams audio for a given song `id`. Relevant parameters:

| Parameter | Description |
|-----------|-------------|
| `id` | Song ID |
| `maxBitRate` | Limit bitrate; 0 = no limit |
| `format` | Target container (`mp3`, `opus`, `aac`, `flac`, `raw`) |
| `timeOffset` | Seek to this position (seconds) before streaming |
| `estimateContentLength` | Hint to help clients display progress |

Navidrome evaluates whether transcoding is needed via `ResolveRequest`:
- If the client can play the native format and no bitrate cap applies, the file is served directly
- Otherwise FFmpeg is invoked with `TranscodeOptions` specifying container, bitrate, sample rate, and seek position

Firmium's `StreamingReader` (`audio/streaming_reader.rs`) keeps the HTTP connection open for the full track duration so Navidrome registers "Now Playing" status correctly, rather than closing after buffering.

### `getCoverArt`

Returns album/artist artwork. Parameters:

| Parameter | Description |
|-----------|-------------|
| `id` | Entity ID (album, artist, song, playlist) |
| `size` | Resize to this pixel dimension (square crop) |

Navidrome sets `cache-control: public, max-age=315360000` (~10 years). Missing artwork returns a placeholder. Firmium maintains its own disk-based cover cache on top of this (`commands/cover_cache.rs`, 200MB budget) to avoid repeat HTTP requests.

### `getLyrics` (legacy)

Matches by artist name + song title. Returns plain text.

### `getLyricsBySongId` (OpenSubsonic extension)

Matches by song ID. Returns structured format with timed lines (LRC-compatible) or plain text. Firmium tries this first, then falls back to legacy, then LRCLIB (see `commands/subsonic.rs::get_song_lyrics()` and `commands/lyrics.rs`).

### Downloads

`download` endpoint streams a ZIP of an album, artist, or playlist. Transcoding may be applied based on client and server config.

---

## OpenSubsonic Extensions

Navidrome advertises supported extensions via `getOpenSubsonicExtensions` and includes the `openSubsonicExtensions` field in every response. Firmium detects this and stores it in the `openSubsonicExtensions` Svelte store. When absent, Firmium degrades gracefully and the Settings page shows a "Subsonic" badge instead of "OpenSubsonic".

Tracking issue for all extensions: [navidrome/navidrome#2695](https://github.com/navidrome/navidrome/issues/2695).

### Implemented Protocol Extensions

Sourced from [`server/subsonic/opensubsonic.go`](https://github.com/navidrome/navidrome/blob/master/server/subsonic/opensubsonic.go) — the authoritative list of what Navidrome returns from `getOpenSubsonicExtensions`.

| Extension | Version | Description |
|-----------|---------|-------------|
| `transcodeOffset` | 1 | Seek to a time offset before transcoding starts, avoiding re-transcoding from zero on resume |
| `formPost` | 1 | Accept parameters via POST form body, not just query string |
| `songLyrics` | 1 | `getLyricsBySongId` — structured/timed lyrics by song ID (see `commands/lyrics.rs`) |
| `indexBasedQueue` | 1 | Play queue saved/restored by index position |
| `transcoding` | 1 | Exposes server transcoding capabilities to clients |
| `playbackReport` | 1 | Report playback progress events to the server |
| `sonicSimilarity` | 1 | `getSonicSimilarTracks` / `findSonicPath` — only advertised when the [AudioMuse](https://audiomuse.ai) WASM plugin is installed and active. Returns HTTP 404 otherwise. |

**Not yet implemented:** `apiKeyAuthentication`, `songLyrics v2`

### Extended Response Fields

These fields are added to standard Subsonic response types when the client is OpenSubsonic-compatible.

#### `Child` (songs/tracks)
| Field | Notes |
|-------|-------|
| `played` | Last played timestamp |
| `bpm` | Beats per minute |
| `comment` | Track comment tag |
| `sortName` | Sort-friendly name |
| `mediaType` | `song`, `podcast`, etc. |
| `musicBrainzId` | MusicBrainz recording ID |
| `genres` | Array of genre objects |
| `artists` | Array of contributing artists |
| `displayArtist` | Pre-formatted artist string for display |
| `albumArtists` | Album artist array |
| `displayAlbumArtist` | Pre-formatted album artist string |
| `contributors` | All contributors with roles |
| `displayComposer` | Formatted composer string |
| `moods` | Mood tags |
| `replayGain` | `trackGain`, `albumGain`, `trackPeak`, `albumPeak` |
| `bitDepth` | Bit depth (e.g. 16, 24) |
| `samplingRate` | Sample rate in Hz |
| `channelCount` | Number of audio channels |
| `explicitStatus` | Explicit content flag |
| `isrc` | ISRC code |
| `groupings` | Grouping/work tags |

Not yet implemented: `works`, `movements`

#### `AlbumID3` (albums)
| Field | Notes |
|-------|-------|
| `played` | Last played timestamp |
| `userRating` | User's 1-5 star rating |
| `recordLabels` | Record label names |
| `musicBrainzId` | MusicBrainz release ID |
| `genres` | Genre array |
| `artists` | Artist array |
| `displayArtist` | Formatted artist string |
| `releaseTypes` | e.g. `["Album"]`, `["Single"]`, `["EP"]` — used by `commands/mappers.rs::infer_release_type()` |
| `moods` | Mood tags |
| `sortName` | Sort-friendly album name |
| `originalReleaseDate` | Original release date (separate from remaster date) |
| `releaseDate` | Release date of this edition |
| `isCompilation` | Compilation flag — used by Android's `ApiClient.kt::inferReleaseType()` |
| `discTitles` | Per-disc title and cover art |
| `explicitStatus` | Explicit content flag |
| `version` | Album version/edition string |

#### `ArtistID3` (artists)
| Field | Notes |
|-------|-------|
| `musicBrainzId` | MusicBrainz artist ID |
| `sortName` | Sort-friendly artist name |
| `roles` | Roles this artist has in the library (e.g. `albumArtist`, `composer`) |

#### `playlist`
| Field | Notes |
|-------|-------|
| `readOnly` | True for smart playlists |
| `validUntil` | When the smart playlist content expires and will be re-evaluated |

#### `internetRadioStation`
| Field | Notes |
|-------|-------|
| `coverArt` | Station artwork ID |

### Fields Used by Firmium

| Field | Where used |
|-------|-----------|
| `replayGain` | Applied during decode in `audio/session.rs` |
| `displayArtist` | Shown in player UI and track listings |
| `releaseTypes[]` | `commands/mappers.rs::infer_release_type()` (desktop) |
| `isCompilation` | `ApiClient.kt::inferReleaseType()` (Android) |
| `genres[]` | Multi-genre display |
| `bpm` | Track info display |
| `musicBrainzId` | Artist/album metadata display |

---

## System Endpoints

| Endpoint | Purpose |
|----------|---------|
| `ping` | Health check; also used to detect OpenSubsonic support |
| `getLicense` | Returns license info (always valid for Navidrome) |
| `getScanStatus` | Returns whether a library scan is in progress + counts |
| `startScan` | Triggers a library rescan |
| `getUser` | Returns the current user's info and roles |
| `getUsers` | Admin only: list all users |

---

## Native REST API (`/api/*`)

Used exclusively by Navidrome's web UI. Firmium does not call these endpoints. Documented here for completeness.

### Standard Resources

| Endpoint | Resource |
|----------|----------|
| `GET /api/song` | Songs (`model.MediaFile`) |
| `GET /api/album` | Albums |
| `GET /api/artist` | Artists |
| `GET /api/user` | Users |
| `GET /api/share` | Shared links (if enabled) |

All support standard REST operations (GET list, GET by ID, POST create, PUT update, DELETE).

### Admin-Only Endpoints

| Endpoint | Purpose |
|----------|---------|
| `/api/config` | Server configuration |
| `/api/plugin` | WASM plugin management |
| `/api/missing` | Files in DB with no matching file on disk |
| `/api/library` | Music library path management |

### Playlist Tracks

Track operations are a sub-resource under `/api/playlist/{id}/tracks`:

- **Add**: accepts song IDs, album IDs, artist IDs, or specific disc identifiers
- **Reorder**: move tracks within the playlist
- **Remove**: batch delete by track IDs

---

## Navidrome-Specific Caveats

These are intentional differences from the standard Subsonic spec to keep in mind when developing:

- **No video** — Navidrome will never implement video endpoints (`getVideos`, `getVideoInfo`, `getVideoStream`, etc.). It is audio-only.
- **Folder browsing is fake** — `getIndexes` and `getMusicDirectory` return simulated trees derived from tags, not real filesystem paths.
- **`scrobble` is the only play tracker** — Navidrome does not mark a song as played when `stream` is called. You must explicitly call `scrobble` with `submission=true` after a track finishes. Firmium does this via `commands/subsonic.rs::scrobble()`.
- **IDs are strings** — Navidrome IDs are always strings (MD5 hashes or UUIDs), never integers. Don't cast them.
- **Search is simple** — `search2`/`search3` use basic substring matching, not Lucene-style queries. Complex query syntax will not work.
- **External integrations are optional** — `getArtistInfo`, `getSimilarSongs`, `getTopSongs`, and `getAlbumInfo` return meaningful data only if Last.fm and/or MusicBrainz are configured server-side. Expect empty or minimal responses otherwise.
- **`getUser` / `getUsers`** — implemented but with limited functionality compared to the full Subsonic spec.

---

## Firmium-Specific Notes

- **Auth token generation**: `src-tauri/src/commands/auth.rs::generate_auth_params()` — produces `t`, `s` for every request
- **Request builder**: `src-tauri/src/commands/subsonic.rs::subsonic_request()` — attaches all required params, handles 401/error 40/41
- **URL builder**: `src/lib/api.ts::OpenSubsonicRouter` — constructs cover art and stream URLs for the frontend
- **Streaming**: `src-tauri/src/audio/streaming_reader.rs` — keeps the connection alive so Navidrome tracks "Now Playing"
- **Cover cache**: `src-tauri/src/commands/cover_cache.rs` — avoids redundant `getCoverArt` calls
- **Lyrics cascade**: `get_song_lyrics()` → OpenSubsonic structured → legacy → LRCLIB
