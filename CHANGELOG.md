# Changelog

## v9.0.2

### What's New
- **Desktop and Android:** The Albums (desktop) and Music (Android) tabs are now a grid of cover tiles grouped into Albums, EPs and Singles instead of a long list. On Android, long-press a tile to add it to a playlist.

- **Desktop:** Servers that require a client certificate (mTLS) now work. Settings > Account lets you add a client certificate, private key and custom CA certificate (PEM files). The private key is stored in your system keyring.
- **Android:** Servers that require a client certificate (mTLS) now work. Add a .p12/.pfx identity (and optionally a custom CA) in the login dialog or Settings > Account. The key is kept in the Android Keystore.

- **Android:** The "Resume server queue?" prompt is now a themed in-app popup instead of the system dialog.

- **Desktop:** The player bar is tidier: Similar Tracks and Audio Stats moved into a three-dot menu, and the equalizer button is gone (the equalizer stays in Settings).

- **Desktop:** Playlists can now have a custom image (click the cover) and a visible rename pencil; Reset image restores the mosaic.
- **Desktop:** Search is now a small icon at the top of the sidebar that is always visible. The server address and logo are gone from there.
- **Desktop:** Genre pages have a back button. Home shows as many Random Picks as fit the window, up to 12.
- **Desktop:** The Spotify UI theme is removed. The equalizer settings no longer show an output device picker (profiles apply to the default device).

### Fixes
- **Desktop:** The play queue is now saved to the server again, so other devices can resume it. The "Resume server queue?" prompt only appears once per launch and only when nothing is playing.
- **Desktop:** Home cover cards no longer flicker on hover, the no-artist photo placeholder is clean, and the empty-cover fallback in the player bar is centered.
- **Desktop:** Shuffle no longer lights up on empty playlists or an empty queue.
- **Android:** Lyrics from LRCLIB now load for tracks with featured artists, "(Live)"/"(Remix)" titles or differing album names, falls back to a fuzzy LRCLIB search when there is no exact match, shows instrumental tracks as such, and the LRCLIB setting is now respected.

## v9.0.1

### What's New
- **Desktop:** The mouse back and forward side buttons now navigate between pages, and going back returns you to your previous scroll position.
- **Desktop:** Opus files now play. Building from source now needs `cmake` and a C compiler.
- **Desktop:** Artist pages have a button to download every album by that artist, with progress.
- **Android:** Swipe a track right to add it to the queue, or long-press it for an Add to queue menu, on album, playlist and favorites lists. The mini player also has a queue button.
- **Android:** Playback pauses automatically when headphones are unplugged or a Bluetooth device disconnects.
- **Desktop:** FreeBSD is no longer mentioned or targeted anywhere in the code and docs.
- **Desktop:** The desktop app is now built on Tauri again. On Linux it needs `webkit2gtk-4.1` installed.
- **Desktop:** Albums can be sorted by name or by recently added ([#65](https://github.com/fossisawesome/firmium/issues/65)).
- **Desktop:** Media keys, headset buttons and the system now-playing widget now control Firmium (MPRIS on Linux, the media overlay on Windows, Now Playing on macOS) ([#69](https://github.com/fossisawesome/firmium/issues/69)).
- **Desktop and Android:** Mix has an **Any** option that plays a random sample of your whole library ([#72](https://github.com/fossisawesome/firmium/issues/72)).
- **Android:** The album list can now be sorted by recently added as well as A–Z ([#65](https://github.com/fossisawesome/firmium/issues/65)).
- **Android:** An F-Droid friendly build without Google libraries is available with `./gradlew assembleRelease -Pfoss`; Wear OS sync is not included in it ([#62](https://github.com/fossisawesome/firmium/issues/62)).
- **Desktop:** Crossfade, gapless, ReplayGain, auto-continue and bit-perfect settings are now remembered between launches. Strict bit-perfect turns gapless off.
- **Desktop:** Podcast episodes play through the main queue and player bar, and your place is saved as you listen.
- **Desktop:** The audio visualizer has been removed.
- **Desktop:** Firmium Recap has been removed. Play history and CSV/JSON export in Settings → Stats stay.
- **Android:** Playback pauses when you unplug headphones or a Bluetooth device disconnects ([#77](https://github.com/fossisawesome/firmium/pull/77)).
- **Android:** The audio visualizer has been removed.
- **Android:** Firmium Recap (phone and TV) has been removed. Play history and export in Settings → Stats stay.
- **Android:** Firmium now saves your queue and position to the server and offers to resume a queue saved by another device when the app opens.
- **Termium:** Press `r` to resume the queue saved on your server by another device.
- **Termium:** Ready-made downloads for Linux, Windows and macOS are now attached to every release, including `.deb`, `.rpm`, a Windows `.exe` installer and a macOS `.dmg`.
- **Website:** The Firmium site has had a polish pass: better phone layout, a working shortcuts overlay, social preview cards, and plain screenshots without fake window frames.
- Desktop installers are now built for `.deb`, `.rpm`, Windows and macOS (`Firmium_*_aarch64.dmg` / `Firmium_*_x64.dmg`).

### Fixes
- Fedora (Copr) build failed with "cannot update the lock file" because `Cargo.lock` still listed version 8.1.0. Lock now matches 9.0.0 and the version bump script keeps it in sync.
- Servers that reject the standard login (such as Nextcloud Music) now work ([#74](https://github.com/fossisawesome/firmium/issues/74)).
- Adding the same track twice to a playlist in one go no longer creates a duplicate.
- Playing the same song twice in a row, or a one-song queue on repeat, no longer cuts off about 30 seconds early.
- A failed crossfade no longer skips ahead while the current song is still playing.
- Playlists and settings can no longer be wiped by a corrupt file. A damaged file is set aside as a backup instead.
- Two saved servers that share cover ids no longer show each other's artwork.
- "Play local copy" and "already downloaded" now match on artist, album and title, so a different song with the same name is no longer swapped in or skipped.
- A failed track download no longer stops the rest of the album.
- A failed login or account switch keeps you signed in to your current server.
- A second copy of the app can no longer take over the command line control.
- Play history exports can no longer run formulas when opened in a spreadsheet.
- **Android:** Imported custom themes accept more color formats, only need background, text and accent colors, and tell you which entries are wrong ([#80](https://github.com/fossisawesome/firmium/issues/80)).
- **Android:** Quoted hex colors such as `"#242424"` in imported themes no longer cause a "missing colors" error ([#80](https://github.com/fossisawesome/firmium/issues/80)).
- **Android:** The shuffle button now works after a crossfade, and turning it on shuffles the upcoming songs ([#76](https://github.com/fossisawesome/firmium/issues/76)).
- **Android:** With shuffle already on, starting an album, playlist or song list now shuffles the rest of the queue instead of playing in order ([#76](https://github.com/fossisawesome/firmium/issues/76)).
- **Android:** Playback now follows an http to https redirect from a reverse proxy, which fixes tracks that would not play behind nginx while browsing worked ([#84](https://github.com/fossisawesome/firmium/issues/84)).
- **Android:** Cover art that fails to load is now logged to logcat under `FirmiumCover` with the reason, to help track down missing covers ([#56](https://github.com/fossisawesome/firmium/issues/56)).
- **Android:** Artist "Play all" and Shuffle now queue the artist's whole catalog instead of stopping at about 20 tracks.
- **Android:** The Podcasts screen no longer overlaps the status bar and matches the other screens.
- **Android:** Podcast episodes now show up in the mini player and full-screen player, with play/pause and 15 second skips, and resume where you left off.

### New Contributors
- Louis_292, welcome aboard ([#77](https://github.com/fossisawesome/firmium/pull/77), [#78](https://github.com/fossisawesome/firmium/pull/78)).
- Ignas Poklad ([#83](https://github.com/fossisawesome/firmium/pull/83)), thank you for your pull request. It targeted the old desktop app, so it could not be merged after the move back to Tauri.
- Uriah ([#82](https://github.com/fossisawesome/firmium/pull/82), [#81](https://github.com/fossisawesome/firmium/pull/81)), thank you for your pull requests. Scroll position restore and the artist Download all button come from your PRs, reimplemented after the move back to Tauri.
- Gabe ([#68](https://github.com/fossisawesome/firmium/pull/68)), thank you for your pull request. It targeted the old desktop app, so it could not be merged after the move back to Tauri.
- Will Hellinger ([#67](https://github.com/fossisawesome/firmium/pull/67), [#63](https://github.com/fossisawesome/firmium/pull/63), [#61](https://github.com/fossisawesome/firmium/pull/61), [#60](https://github.com/fossisawesome/firmium/pull/60)), thank you for your pull requests. The mouse back/forward buttons and Opus playback come from your PRs; the others targeted the old desktop app and could not be merged after the move back to Tauri.

## v8.1.0 (2026-07-15)

### What's New
- A Favorites screen on desktop and Android, with a favorite button throughout the app.
- Home screen and player bar refreshed with a Spotify-style layout.
- **Termium:** Firmium now has a terminal app with Home, Albums, Artists, Playlists and Search screens, a player bar and its own keymap.
- A privacy policy now covers every Firmium app.

## v8.0.0 (2026-07-05)

### What's New
- **Wear OS:** Firmium now has a full Wear OS app. Browse and play your library right from the watch, with no phone needed.
- **Wear OS:** Search by voice or text, control volume with the crown, and download tracks for offline listening.
- **Wear OS:** Theme, crossfade, gapless, ReplayGain and download settings sync from your phone, and the watch remembers whether you last used it standalone or as a remote.
- **Wear OS:** Your account signs in on the watch automatically when you log in on the phone.
- **Android TV:** Firmium now runs on Android TV with remote-friendly screens for Home, Albums, Artists, Playlists and Search.
- **Android TV:** Full-screen Now Playing with queue, synced lyrics and similar tracks, plus Smart Mix, Equalizer, Recap and Settings.
- **Desktop and Android:** A new Spotify-style layout that you pick separately from the color theme, plus a matching Spotify color theme.
- **Desktop and Android:** The visualizer particle count can be changed, and the Android visualizer was reworked.

## v7.1.0 (2026-07-02)

### What's New
- **Desktop:** Control a running Firmium from the terminal with commands like `firmium play-pause`, `next`, `volume`, `seek` and `status`. Handy for scripts and keyboard shortcuts.
- **Desktop:** A scrollbar width slider in Appearance settings.
- **Desktop:** Cover art for the queue and playlists loads ahead of time instead of as you scroll.
- **Desktop:** The account overlay has a close button.
- **Android:** A Play button on the artist page starts that artist's top tracks.

### Fixes
- **Desktop:** Libraries with more than 500 albums now show every album.
- **Android:** Failed scrobbles are now logged instead of silently ignored.

## v7.0.0 (2026-06-29)

This is a major release. The desktop app was rebuilt from scratch as a native Rust app, so expect some rough edges and please report anything you find.

### What's New
- **Desktop:** A brand new native desktop app. No web view, and settings now live in a single config file that remembers your server, username, theme, volume and saved accounts.
- **Desktop:** Pick your UI font in Appearance settings.
- **Desktop:** The Windows installer was rebuilt for the new app.
- **Android:** Firmium Recap with swipeable cards for top tracks, artists, albums and genre, time-of-day and weekday habits, your biggest discovery and your listening streak.
- **Android:** Listening history is now saved on your device.
- **Android:** The now-playing notification shows shuffle and repeat states.
- **Android:** Import a custom theme from a `.toml` file.
- **Android:** Share Recap cards through the share sheet.
- **Android:** Pick your UI font, matching desktop.

## v6.6.0 (2026-06-19)

### What's New
- **Android and Desktop:** Switch between saved servers without retyping your password.
- **Android and Desktop:** Rate tracks from 1 to 5 stars.
- **Android and Desktop:** Filter the album list by decade and genre.
- **Android and Desktop:** Filter album tracks by tempo (BPM). Desktop also adds this to playlists.
- **Android:** A Favorite button on the now-playing notification.

## v6.5.0 (2026-06-18)

### What's New
- **Android:** A Wear OS remote control app shows the current track and lets you play, pause, skip and change volume from your watch.
- **Android:** Your server library and downloaded local music are now merged into one library, including on the home screen.

### Fixes
- **Android:** Repeat-all now restarts the queue after the last track.
- **Android:** Repeat and auto-next no longer stop working when the app is in the background.
- **Android:** The visualizer reacts less to heavy bass and shows fewer, cleaner bars.

## v6.4.1 (2026-06-18)

### What's New
- A full list of every feature is now in `FEATURES.md`.

### Fixes
- **Android:** The app icon now fits properly on all Android 8+ launchers.

## v6.4.0 (2026-06-18)

### What's New
- **Android and Desktop:** A short welcome tour on first launch.
- **Android and Desktop:** Playlists show a mosaic of up to four covers.
- **Android and Desktop:** A new dark blue theme called Svalbard.
- **Android:** Three visualizer styles (Orb, Bars, Oscilloscope). Tap the visualizer to switch, or turn it off in Settings.
- **Android:** An add-to-playlist button on the album art in the full-screen player, with shuffle and repeat moved next to the main controls.
- **Android:** Remove a track directly from a playlist.
- **Android:** The notification and Android Auto are tinted to match the cover art.
- **Android Auto:** Browse music A to Z, shuffle any playlist in one tap, and see an Up Next list.
- **Android Auto:** Steering wheel controls now work for shuffle and repeat.
- Updated app icons on every platform.

### Fixes
- **Android:** The notification no longer shows 0:00 when you pause.
- **Android:** The notification uses a proper status bar icon.
- **Android:** Several permission requests at once now all show up instead of only the first.

## v6.3.0 (2026-06-17)

### What's New
- **Android Auto:** Firmium now works as a media app in the car. Browse Home, Albums, Artists and Playlists, search by voice, and use steering wheel controls. Sideloaded installs need "Unknown sources" turned on in Android Auto's developer settings (see the [Android Auto guide](https://docs.firmium.app/android-auto)).
- **Android and Desktop:** ReplayGain can be switched on or off at any time without restarting playback.

## v6.2.0 (2026-06-17)

### What's New
- **Desktop:** Queue handling is smoother and more reliable, and your queue is saved to the server as you play.
- **Desktop:** Colors drawn from cover art are now extracted much faster.

### Fixes
- **Desktop:** Fixed rare cases where crossfades or track changes could miss or double a scrobble.

## v6.1.6 (2026-06-17)

### What's New
- **Desktop:** Bit-perfect audio with Off, Relaxed and Strict modes. Relaxed matches each track's sample rate when it can, and Strict also turns crossfade off.

### Fixes
- **Android:** Playback no longer goes silent on Samsung phones when the visualizer is on.
- **Android:** The Android bit-perfect setting was removed.

## v6.1.5 (2026-06-16)

### What's New
- **Android:** A warning appears when you connect to a non-local server over plain HTTP.

### Fixes
- **Android:** Plain-HTTP servers can be reached again in release builds.
- **Android:** Bad responses from a server or reverse proxy now show a clear error instead of crashing.
- **Android:** The orb no longer freezes when you pause.
- **Android:** Fixed a rare crash when adding a song to a playlist.
- **Desktop:** Fixed a rare crash when seeking near the end of very large files.

## v6.1.4 (2026-06-16)

### What's New
- **Desktop:** A smoother, more responsive visualizer with a wider frequency range.
- **Android:** The orb now cycles through all colors from the cover art.

## v6.1.3 (2026-06-16)

### What's New
- **Desktop:** The orb visualizer takes its colors from the current cover art and has richer visuals.
- **Desktop:** Streamed tracks play your downloaded copy when you have one, and tracks you already have are not downloaded again.
- **Android:** Bit-perfect audio mode with Off, Relaxed and Strict.

### Fixes
- **Desktop:** Auto-login now handles missing or failed saved passwords by showing the account window.

## v6.1.2 (2026-06-15)

### What's New
- **Android and Desktop:** Shuffle play for playlists.
- **Android and Desktop:** Move tracks up or down within a playlist.

### Fixes
- New playlists synced to the server now include their tracks from the start.
- Playlists with a single track now load on servers that handle one-item lists differently.

## v6.1.1 (2026-06-15)

### What's New
- **Android and Desktop:** A Sync button pushes a local playlist to your server on demand.
- **Desktop:** The add-to-playlist menu now lists server playlists made on other devices.
- **Desktop:** Cloud icons mark which playlists are synced.

## v6.1.0 (2026-06-15)

### What's New
- **Android and Desktop:** Karaoke-style lyrics that fill in word by word, with a toggle to go back to plain line highlighting.
- **Android and Desktop:** The lyrics view glows in the color of the current cover art.
- **Android and Desktop:** Playlists now sync with your server, and local and server playlists appear in one list.
- **Desktop:** A new Firmium logo in the sidebar.
- **Desktop:** The seek bar reacts faster, and seeking in OGG and MP4 files works better.
- **Desktop:** Better support for screen readers and keyboard navigation in the player, account window, sidebar and playlist menu.
- **Android:** Sliders and switches are easier to tap.

### Fixes
- **Desktop:** Gapless switching no longer keeps the wrong track playing when a resumed queue is reordered.

## v6.0.0 (2026-06-14)

### What's New
- **Desktop:** The audio engine was rebuilt from the ground up, with better sound quality and device handling.

### Fixes
- **Desktop:** The old bit-perfect setting was removed because the new engine matches each track to your device automatically.

## v5.5.0 (2026-06-14)

### What's New
- **Desktop:** Offline mode. With no server connected, Firmium plays music from `~/Music/Firmium`.
- **Desktop:** Drag audio files or folders onto the window to add them to your library.
- **Android and Desktop:** Download tracks and albums from your server, in original quality or a converted format.
- **Desktop:** An audio visualizer with an orb mode and a bars mode.
- **Android and Desktop:** Resume your queue where you left off on another device.
- **Android and Desktop:** A new account window for signing in, switching and disconnecting.
- **Desktop:** The sidebar shrinks to icons on narrow windows and becomes a bottom tab bar on phones and tablets.

## v5.4.0 (2026-06-13)

### What's New
- **Android and Desktop:** Similar Tracks now works on servers without sonic similarity by using genres and similar artists.

### Fixes
- Server feature detection now works correctly.

## v5.3.0 (2026-06-13)

### What's New
- **Android and Desktop:** Firmium reports what you are playing to servers that support it.
- **Android and Desktop:** Similar Tracks panel on servers that support sonic similarity.

### Fixes
- Fixed a crash when playing music ([#20](https://github.com/fossisawesome/firmium/issues/20)).

## v5.2.0 (2026-06-12)

### What's New
- **Desktop:** Cover art is saved on your disk, so it loads instantly and survives restarts.
- **Desktop:** Lyrics, server calls and track format details are faster and more consistent.

## v5.1.0 (2026-06-12)

### What's New
- **Desktop:** Bit-perfect audio output that matches each track's native sample rate.
- **Android and Desktop:** The player shows each track's format, such as "FLAC · 96 kHz · 24-bit · 1411 kbps".
- **Android:** "Delete Logs" is now "Clear Cache".

### Fixes
- The debug flag and in-app log files were removed.

## v5.0.0 (2026-06-11)

### What's New
- **Desktop:** Long album, artist and playlist lists scroll much more smoothly.
- **Desktop:** In-app updates for Windows and Linux AppImage builds.
- Refreshed app icons on every platform.
- Automated security scanning and dependency updates, plus a contributor guide.

## v4.0.1 (2026-05-30)

### Fixes
- **Android:** Swiping down from the top of the full-screen player now reliably closes it.
- **Android:** Bottom navigation always works, including from an artist page opened in another tab.
- **Android:** Pressing play after the queue ends restarts the track.
- **Android:** Lock screen and headset controls keep working after the queue ends.

## v4.0.0 (2026-05-30)

### What's New
- Android is now a separate, fully native app. The desktop app is desktop only.

## v3.1.6 (2026-05-29)

### Fixes
- **Android:** The app stays in sync when it returns to the foreground after tracks changed in the background.
- **Android:** Swipe-to-close only works from the top handle.
- **Desktop:** Pausing no longer causes brief stalls.
- Fixed the highlighted lyric line sticking to the previous song after a track change.

## v3.1.5 (2026-05-28)

### Fixes
- Search section headers now line up with the list.

## v3.1.4 (2026-05-28)

### What's New
- **Android:** Artist bios are hidden behind a "Show Bio" button.
- Fedora packages are now published automatically with each release.

### Fixes
- **Android:** Search closes when you play a track from it.
- **Android:** Artist and tracklist header layouts were tidied.

## v3.1.3 (2026-05-28)

### What's New
- **Desktop:** Smoother, more accurate progress tracking.
- **Desktop:** Faster, lighter app with fewer background components.

### Fixes
- The System info row was removed from Settings.

## v3.1.0 (2026-05-28)

### What's New
- **Android:** Tracks now advance reliably even when the app is in the background.
- **Android:** Full-screen search and settings, with settings grouped into collapsible sections.
- **Android:** Back button and gesture now step through search, settings, queue and player before leaving the app.
- **Android:** Page titles with quick Search and Settings icons.
- **Android:** A round play button on album and playlist pages.
- **Android:** Tap the album art to show lyrics, and an add-to-playlist button replaces the lyrics shortcut.
- **Android:** The queue opens on top of the player.
- **Android and Desktop:** Artist photos in the artist list and on the home screen.

### Fixes
- **Android:** Swiping on the cover art no longer opens lyrics by accident.
- A possible fix for [#3](https://github.com/fossisawesome/firmium/issues/3).

## v3.0.0 (2026-05-27)

### What's New
- **Android:** First mobile version, with a full-screen player and a queue sheet.
- Custom themes: drop theme files into the `themes/` folder and pick them in Settings.
- Clean icons replace emoji throughout the app.
- Smooth fade-in when a track starts, and a gentle fade on pause and stop.

### Fixes
- Fixed the pop at the start of each track and the click on pause or stop.
- A failed login no longer leaves the app half signed in.

## v2.1.0 (2026-05-26)

### What's New
- **Gapless playback:** the next song is prepared ahead of time so there is no gap between tracks.
- **ReplayGain:** volume is evened out across tracks.
- Smoother crossfades.
- Last.fm artist biographies, which can be turned on in Settings.
- New themes: Monokai Classic, Monokai Pro, Adwaita, Adwaita Dark, ayu, ayu Light, GitHub Dark, Nordfox and Synthwave '84.
- Fedora and RHEL packaging.

### Fixes
- Home now shows one row of recently played albums, and artist cards open the artist page.
- Track duration no longer shows 0 at the start of a song.
- Failed cover art loads no longer cause errors.
- Wikipedia biographies were replaced by Last.fm because they were unreliable.

## v2.0.0 (2026-05-24)

### What's New
- The app was rewritten with a new foundation, making it faster to update and more responsive.

## v1.6.0 (2026-05-24)

### What's New
- Full Windows support, with an installer.
- Smaller app icons.

## v1.5.1 (2026-05-24)

### Fixes
- Plain-HTTP servers such as Navidrome on `http://localhost:4533` now connect correctly.
- Fixed playback event and state issues.

### New Contributors
- foogs, welcome aboard.

## v1.5.0 (2026-05-24)

### What's New
- Lyrics now match much more often for remixes, covers and featured-artist tracks.

### Fixes
- Arch Linux packaging handles name changes and avoids duplicate launcher entries.

## v1.4.0 (2026-05-24)

### What's New
- Synced lyrics in a sliding side panel, with the current line highlighted as the song plays. Plain lyrics and instrumental tracks are handled too.
- A lyrics button in the playback controls.

### Fixes
- `http` addresses now work as a server URL.

## v1.3.0 (2026-05-24)

### What's New
- Crossfade between tracks, from 1 to 12 seconds.
- The app version is shown in Settings.
- The interface got a visual refresh.

## v1.2.1 (2026-05-23)

### Fixes
- Smoother progress tracking during playback.
- Fixed a possible freeze when seeking.
- Progress tracking stops cleanly at the end of a track.

## v1.2.0 (2026-05-23)

### What's New
- Detection of OpenSubsonic server features.
- Safer sign-in handling.

### Fixes
- Fixed audio streaming connection and thread-safety problems.

## v1.0.0 (2026-05-21)

### What's New
- The first stable release, for Arch, Debian and Fedora based Linux distributions.
- A fast native audio engine that streams MP3, OGG, FLAC, WAV and more.
- Passwords are stored in your system keyring, never in plain text.
- Eight color themes: Firmium, Gruvbox, Tokyo Night, Dracula and four Catppuccin flavors.
- Optional artist biographies in Settings.

### Fixes
- The window decoration setting is now remembered.
- Fixed keyring compatibility, the logout button and audio stability.
