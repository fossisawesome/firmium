// ============================================================================
// OPENSUBSONIC AUTH
// ============================================================================

use std::sync::atomic::{AtomicBool, Ordering};

/// Some servers (e.g. Nextcloud Music) only accept the legacy `p=enc:<hex>` password
/// param, not the MD5 token. Set by `validate_connection` when token auth is rejected.
static LEGACY_AUTH: AtomicBool = AtomicBool::new(false);

pub fn set_legacy_auth(enabled: bool) {
    LEGACY_AUTH.store(enabled, Ordering::Relaxed);
}

fn legacy_auth_params(username: &str, password: &str) -> serde_json::Value {
    use std::fmt::Write as _;
    let hex = password.bytes().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    });
    serde_json::json!({
        "u": username,
        "p": format!("enc:{hex}"),
        "v": "1.16.1",
        "c": "firmium",
        "f": "json"
    })
}

/// Generate OpenSubsonic token-auth query parameters on the Rust side.
/// Keeps MD5 out of the JS layer — the frontend passes plaintext credentials
/// and receives the ready-to-use param map.
pub fn generate_auth_params(username: String, password: String) -> serde_json::Value {
    use std::fmt::Write as _;
    if LEGACY_AUTH.load(Ordering::Relaxed) {
        return legacy_auth_params(&username, &password);
    }
    // Cryptographically random 8-byte salt, hex-encoded.
    let salt_bytes: [u8; 8] = rand::random();
    let mut salt = String::with_capacity(16);
    for b in salt_bytes {
        let _ = write!(salt, "{:02x}", b);
    }
    let token = format!("{:x}", md5::compute(format!("{password}{salt}")));
    serde_json::json!({
        "u": username,
        "t": token,
        "s": salt,
        "v": "1.16.1",
        "c": "firmium",
        "f": "json"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_md5_of_password_and_returned_salt() {
        let params = generate_auth_params("alice".to_string(), "hunter2".to_string());
        let salt = params["s"].as_str().unwrap();
        let expected_token = format!("{:x}", md5::compute(format!("hunter2{}", salt)));
        assert_eq!(params["t"], expected_token);
    }

    #[test]
    fn salt_is_16_hex_chars_and_random_per_call() {
        let a = generate_auth_params("alice".to_string(), "hunter2".to_string());
        let b = generate_auth_params("alice".to_string(), "hunter2".to_string());
        let salt_a = a["s"].as_str().unwrap();
        let salt_b = b["s"].as_str().unwrap();

        assert_eq!(salt_a.len(), 16);
        assert!(salt_a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(salt_a, salt_b);
        assert_ne!(a["t"], b["t"]);
    }

    #[test]
    fn legacy_auth_sends_hex_encoded_password_without_token() {
        let params = legacy_auth_params("alice", "hunter2");
        assert_eq!(params["p"], "enc:68756e74657232");
        assert!(params.get("t").is_none() && params.get("s").is_none());
    }

    #[test]
    fn includes_fixed_protocol_fields() {
        let params = generate_auth_params("alice".to_string(), "hunter2".to_string());
        assert_eq!(params["u"], "alice");
        assert_eq!(params["v"], "1.16.1");
        assert_eq!(params["c"], "firmium");
        assert_eq!(params["f"], "json");
    }
}
