# Privacy Policy

_Last updated: 2026-07-14_

Firmium is a local-first music and podcast player. This policy covers the desktop app, Android app, Wear OS companion, and Termium (TUI).

Firmium is developed by an individual maintainer, not a registered company. This policy has no designated governing-law jurisdiction — if you require one for compliance purposes, contact the developer to discuss.

## Summary

- No analytics, telemetry, ads, or crash reporting is built into Firmium.
- No account or sign-up is required to use Firmium.
- Your music library, playback history, and settings stay on your device (or on a media server you connect, like Navidrome/Subsonic) unless a feature below sends a request elsewhere.
- Firmium does not sell or share your data. There is no data to sell — nothing is collected centrally.

## Data stored locally

Firmium stores the following on your device only:

- Library metadata (scanned from your local files or your media server)
- Playback queue, history, and listening stats (used for the Recap feature)
- Playlists
- App settings and themes
- Cached cover art and lyrics (to avoid repeat network requests)
- Downloaded podcast episodes and tracks (offline mode)

This data is never transmitted to Firmium's developers. There are no Firmium-operated servers.

## Network requests (only when you use the feature)

Firmium is offline by default. It makes outbound requests only for features you actively use:

| Feature | Service contacted | Data sent |
|---|---|---|
| Connecting to a media server | Your Navidrome/Subsonic server (self-hosted or third-party, address you provide) | Your server credentials and library requests |
| Lyrics | [lrclib.net](https://lrclib.net) | Track title/artist/duration to look up lyrics |
| Similar tracks / artist bios | [ws.audioscrobbler.com](https://ws.audioscrobbler.com) (Last.fm) | Track/artist metadata |
| Scrobbling (optional, off by default) | [api.listenbrainz.org](https://api.listenbrainz.org) | Your ListenBrainz token and play history, only if you enable it and provide a token |
| Podcast subscriptions | The podcast's RSS feed URL and host | Feed fetch requests |
| Update checks / release info | [github.com](https://github.com) | Anonymous request for latest release metadata |

None of these requests include your local library contents beyond the specific track/artist metadata needed for the lookup. Credentials you enter (media server, ListenBrainz token) are stored locally on your device and sent only to the service they're for.

## Data retention

Locally stored data (library metadata, history, cache, downloads) persists on your device until you delete it (uninstall, clear app data, or use in-app clear/reset options) — Firmium sets no automatic expiry. Data you send to a third-party service (your media server, ListenBrainz, Last.fm) is retained per that service's own policy, not Firmium's.

## Your rights (GDPR / CCPA / similar)

Firmium itself holds no personal data on any server, so there is nothing for the developer to export, correct, or delete on your behalf — all data described above lives on your device and is under your direct control at all times (delete via app settings, clear app data, or uninstall).

If you are in the EU/UK, California, or another jurisdiction with statutory data-subject rights (access, correction, deletion, portability, opt-out of sale — noting Firmium sells nothing), those rights apply against the third-party services you choose to connect (media server operator, ListenBrainz, Last.fm, podcast host), not against Firmium. Contact those services directly for requests concerning data you sent them.

If you believe Firmium's local-only design is inaccurate for some feature, or want to raise a rights request anyway, use the contact method below and it will be addressed on a best-effort basis.

## Android permissions

- `INTERNET` — for the network requests above
- `READ_MEDIA_AUDIO` / `READ_EXTERNAL_STORAGE` — to read your local music library
- `FOREGROUND_SERVICE`, `FOREGROUND_SERVICE_MEDIA_PLAYBACK` — to keep playback running in the background
- `POST_NOTIFICATIONS` — to show the playback notification

None of these permissions are used to collect or transmit data beyond what's described above.

## Third-party services

If you connect Firmium to a third-party service (your own Subsonic/Navidrome server, ListenBrainz, Last.fm, a podcast host), that service's own privacy policy governs data you send to it. Firmium is not responsible for third-party data handling.

## Children's privacy

Firmium is not directed at children and does not knowingly collect personal information from anyone, including children under 13 (COPPA) or 16 (GDPR), since the app collects no data by default and requires no account or profile to use. If a parent or guardian believes a child has provided personal information through a connected third-party service (e.g. entered credentials into a media server), that concern should be raised with the operator of that service.

## App store data safety disclosures

For Google Play's Data Safety form and Apple's App Privacy (nutrition label) declarations: Firmium collects no data, does not share data with the developer or any analytics/advertising SDK, and does not link data to user identity on any Firmium-operated system. Data entered for third-party integrations (server credentials, ListenBrainz token) is stored locally in the OS keyring/app storage and transmitted only to the service it configures, per the table above. These declarations will be kept in sync with this policy on each store submission.

## Changes to this policy

Material changes to Firmium's data practices will be reflected here and in the corresponding docs page with an updated date at the top. Continued use of the app after a change constitutes acceptance of the revised policy. Check this page periodically if you have ongoing concerns.

## Contact

Privacy questions or rights requests: Discord `me.kt`, or open an issue on the project's GitHub repository. Response is best-effort, individual-maintainer basis — no dedicated legal/support team.
