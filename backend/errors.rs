// ============================================================================
// USER-FACING ERRORS
// ============================================================================
// One place that turns technical failures into short, friendly messages.
// Internal code keeps returning `Result<_, String>`; conversion to `UserError`
// happens where a live error source exists (subsonic_request) or, as a last
// resort, by classifying a legacy error string via `classify`.

/// A user-facing error category. `message()` is the only place wording lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserError {
    Network,
    Timeout,
    /// Initial-login auth failure (bad credentials). Mid-session expiry uses
    /// `SessionExpired` instead so it can defer to the existing logout flow.
    Auth,
    /// Mid-session 401 / OpenSubsonic 40/41. The toast layer ignores this
    /// variant; the `SessionExpired` event already drives the UI.
    SessionExpired,
    NotFound,
    Server { code: u16 },
    Storage,
    Unknown,
}

impl UserError {
    pub fn message(&self) -> String {
        match self {
            UserError::Network => "Can't reach your server. Check your connection.".into(),
            UserError::Timeout => "The server took too long to respond. Try again.".into(),
            UserError::Auth => "Login failed. Check your username and password.".into(),
            UserError::SessionExpired => "Your session expired. Please log in again.".into(),
            UserError::NotFound => "That item couldn't be found on the server.".into(),
            UserError::Server { code } => format!("The server reported a problem (code {code}). Try again later."),
            UserError::Storage => "Couldn't access your device's secure storage.".into(),
            UserError::Unknown => "Something went wrong. Please try again.".into(),
        }
    }

    /// Last-resort classification for legacy `String` errors that no longer
    /// carry a typed source. Match only on stable, self-produced prefixes.
    pub fn classify(s: &str) -> UserError {
        let l = s.to_lowercase();
        if s == "SESSION_EXPIRED" {
            UserError::SessionExpired
        } else if l.contains("not connected") {
            UserError::Network
        } else if l.contains("timed out") || l.contains("timeout") {
            UserError::Timeout
        } else if l.contains("keyring") || l.contains("secret") || l.contains("keystore") {
            UserError::Storage
        } else {
            UserError::Unknown
        }
    }
}

impl From<reqwest::Error> for UserError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            UserError::Timeout
        } else if e.is_connect() {
            UserError::Network
        } else if let Some(status) = e.status() {
            UserError::Server { code: status.as_u16() }
        } else {
            UserError::Network
        }
    }
}

impl From<std::io::Error> for UserError {
    fn from(_: std::io::Error) -> Self {
        UserError::Storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_maps_known_prefixes() {
        assert_eq!(UserError::classify("SESSION_EXPIRED"), UserError::SessionExpired);
        assert_eq!(UserError::classify("Not connected"), UserError::Network);
        assert_eq!(UserError::classify("operation timed out"), UserError::Timeout);
        assert_eq!(UserError::classify("keyring locked"), UserError::Storage);
        assert_eq!(UserError::classify("weird engine message"), UserError::Unknown);
    }

    #[test]
    fn io_error_is_storage() {
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "x");
        assert_eq!(UserError::from(io), UserError::Storage);
    }

    #[test]
    fn messages_are_nonempty_for_every_variant() {
        for v in [
            UserError::Network, UserError::Timeout, UserError::Auth,
            UserError::SessionExpired, UserError::NotFound,
            UserError::Server { code: 500 }, UserError::Storage, UserError::Unknown,
        ] {
            assert!(!v.message().is_empty());
        }
    }
}
