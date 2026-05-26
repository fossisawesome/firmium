# Security Policy

## Supported Versions

Only the latest release receives security fixes.

| Version | Supported |
| ------- | --------- |
| 2.x.x   | ✓         |
| < 2.0   | ✗         |

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Report vulnerabilities by emailing the maintainer directly or opening a [GitHub Security Advisory](https://github.com/fossisawesome/firmium-desktop/security/advisories/new) (private disclosure).

Include:
- A description of the vulnerability and its potential impact
- Steps to reproduce or a proof-of-concept
- Affected version(s)

You can expect an acknowledgement within 72 hours and a resolution timeline within 14 days for confirmed issues.

## Security Design

### Credential Handling
- Passwords are stored in the **OS keyring** (libsecret on Linux), never in localStorage or any file on disk.
- Plaintext passwords are passed once to the Rust backend via Tauri IPC; MD5 token hashing happens on the Rust side so credentials never appear in JavaScript or network logs.

### Network
- All Subsonic API requests use MD5-hashed tokens (`t` + `s` salt), not plaintext passwords.
- The app targets local/self-hosted servers. TLS is recommended for any non-loopback server address.
- Content-Security-Policy allows `http://*` to support local servers without HTTPS; users connecting to remote servers should use HTTPS.

### Tauri Permissions
- Tauri command access is scoped via `src-tauri/capabilities/default.json`. Only explicitly listed commands are callable from the frontend.
- No `shell` or `fs` write access beyond cover art caching to a defined cache directory.

### Audio Streaming
- Audio is streamed directly from the configured Subsonic server; no audio data is written to disk.
- Cover art is cached to disk via `cache_cover()` in a controlled location.

## Out of Scope

- Vulnerabilities requiring physical access to the machine
- Social engineering attacks
- Issues in self-hosted Subsonic/Navidrome server software (report those upstream)
