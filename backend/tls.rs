//! Shared HTTP clients with optional mTLS: a client identity (certificate +
//! private key) and/or an extra CA root. Certificates are PEM files under
//! `<config>/certs/`; the private key lives in the OS keyring (never on disk).
//! Clients are rebuilt live when any of it changes.

use crate::commands::credentials;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::OnceLock;

const CLIENT_CERT: &str = "client.pem";
const KEY_NAME: &str = "tls_client_key";
// Windows Credential Manager caps a secret near 2.5 KB, smaller than an RSA key PEM.
const KEY_CHUNK: usize = 1000;
const CA_CERT: &str = "ca.pem";

struct Clients {
    http: reqwest::Client,
    blocking: reqwest::blocking::Client,
}

fn slot() -> &'static RwLock<Clients> {
    static SLOT: OnceLock<RwLock<Clients>> = OnceLock::new();
    SLOT.get_or_init(|| {
        // A broken saved cert must not brick startup: fall back to plain TLS.
        let c = build().unwrap_or_else(|e| {
            eprintln!("mTLS config ignored: {e}");
            build_with(None, None).expect("failed to build http client")
        });
        RwLock::new(c)
    })
}

/// Shared async client (cheap clone).
pub fn http() -> reqwest::Client {
    slot().read().http.clone()
}

/// Shared blocking client for the playback thread (cheap clone).
pub fn blocking_http() -> reqwest::blocking::Client {
    slot().read().blocking.clone()
}

fn certs_dir() -> PathBuf {
    let dir = crate::paths::config_dir().join("certs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn file_name(kind: &str) -> Result<&'static str, String> {
    match kind {
        "clientCert" => Ok(CLIENT_CERT),
        "clientKey" => Ok(KEY_NAME),
        "caCert" => Ok(CA_CERT),
        other => Err(format!("unknown cert kind: {other}")),
    }
}

/// Which certs are currently installed: (client cert, client key, CA).
pub fn installed() -> (bool, bool, bool) {
    let d = certs_dir();
    (d.join(CLIENT_CERT).exists(), read_key().is_some(), d.join(CA_CERT).exists())
}

fn chunk_name(i: usize) -> String {
    format!("{KEY_NAME}.{i}")
}

fn read_key() -> Option<Vec<u8>> {
    let n: usize = credentials::get_password(None, &chunk_name(0)).ok()?.parse().ok()?;
    let mut out = String::new();
    for i in 1..=n {
        out.push_str(&credentials::get_password(None, &chunk_name(i)).ok()?);
    }
    Some(out.into_bytes())
}

fn clear_key() {
    if let Some(Ok(n)) = credentials::get_password(None, &chunk_name(0)).ok().map(|c| c.parse::<usize>()) {
        for i in 1..=n {
            let _ = credentials::delete_password(None, &chunk_name(i));
        }
    }
    let _ = credentials::delete_password(None, &chunk_name(0));
}

fn store_key(pem: &str) -> Result<(), String> {
    clear_key();
    // PEM is ASCII, so byte chunks are valid str slices.
    let chunks: Vec<&str> = pem.as_bytes().chunks(KEY_CHUNK).map(|c| std::str::from_utf8(c).unwrap_or_default()).collect();
    for (i, c) in chunks.iter().enumerate() {
        credentials::save_password(None, &chunk_name(i + 1), c).map_err(|e| format!("keyring: {e}"))?;
    }
    credentials::save_password(None, &chunk_name(0), &chunks.len().to_string()).map_err(|e| format!("keyring: {e}"))
}

/// Splits PEM text into (everything except private-key blocks, private-key blocks).
fn split_key(pem: &str) -> (String, String) {
    let (mut rest, mut key) = (String::new(), String::new());
    let mut in_key = false;
    for line in pem.lines() {
        if line.starts_with("-----BEGIN") {
            in_key = line.contains("PRIVATE KEY");
        }
        let out = if in_key { &mut key } else { &mut rest };
        out.push_str(line);
        out.push('\n');
        if line.starts_with("-----END") {
            in_key = false;
        }
    }
    (rest, key)
}

fn read(name: &str) -> Option<Vec<u8>> {
    std::fs::read(certs_dir().join(name)).ok()
}

fn build() -> Result<Clients, String> {
    let identity = read(CLIENT_CERT).map(|mut pem| {
        if let Some(key) = read_key() {
            pem.push(b'\n');
            pem.extend(key);
        }
        pem
    });
    build_with(identity, read(CA_CERT))
}

fn build_with(identity: Option<Vec<u8>>, ca: Option<Vec<u8>>) -> Result<Clients, String> {
    let identity = identity
        .map(|pem| reqwest::Identity::from_pem(&pem).map_err(|e| format!("client certificate: {e}")))
        .transpose()?;
    let roots = match ca {
        Some(pem) => {
            let roots = reqwest::Certificate::from_pem_bundle(&pem).map_err(|e| format!("CA certificate: {e}"))?;
            // A non-PEM file parses to zero certs; refuse it rather than silently trust nothing.
            if roots.is_empty() {
                return Err("CA certificate: no PEM certificates found".into());
            }
            roots
        }
        None => Vec::new(),
    };

    let mut a = reqwest::Client::builder().user_agent("Firmium");
    let mut b = reqwest::blocking::Client::builder().user_agent(concat!("firmium-desktop/", env!("CARGO_PKG_VERSION")));
    for r in roots {
        a = a.add_root_certificate(r.clone());
        b = b.add_root_certificate(r);
    }
    if let Some(id) = identity {
        a = a.identity(id.clone());
        b = b.identity(id);
    }
    // Verification stays on; pinned explicitly so a refactor can't weaken it.
    Ok(Clients {
        http: a
            .danger_accept_invalid_certs(false)
            .danger_accept_invalid_hostnames(false)
            .build()
            .map_err(|e| e.to_string())?,
        blocking: b
            .danger_accept_invalid_certs(false)
            .danger_accept_invalid_hostnames(false)
            .build()
            .map_err(|e| e.to_string())?,
    })
}

fn rebuild() -> Result<(), String> {
    let new = build()?;
    let old = std::mem::replace(&mut *slot().write(), new);
    // Dropping a blocking client inside an async context panics; drop it elsewhere.
    std::thread::spawn(move || drop(old));
    Ok(())
}

/// Installs the PEM at `src` and rebuilds the clients. A private key (a
/// `clientKey` file, or a key block inside a `clientCert` file) goes to the
/// keyring, not disk. On a bad file the previous state is restored and the
/// error returned.
pub fn import(kind: &str, src: &std::path::Path) -> Result<(), String> {
    file_name(kind)?;
    let text = String::from_utf8(std::fs::read(src).map_err(|e| e.to_string())?)
        .map_err(|_| "not a PEM file".to_string())?;
    let (old_cert, old_key, old_ca) = (read(CLIENT_CERT), read_key(), read(CA_CERT));
    let res = (|| {
        match kind {
            "clientKey" => store_key(&text)?,
            "clientCert" => {
                let (cert, key) = split_key(&text);
                write_private(&certs_dir().join(CLIENT_CERT), cert.as_bytes())?;
                if !key.is_empty() {
                    store_key(&key)?;
                }
            }
            _ => write_private(&certs_dir().join(CA_CERT), text.as_bytes())?,
        }
        rebuild()
    })();
    if res.is_err() {
        // Restore the previous state wholesale.
        restore(CLIENT_CERT, old_cert);
        restore(CA_CERT, old_ca);
        match old_key {
            Some(k) => {
                let _ = store_key(&String::from_utf8_lossy(&k));
            }
            None => clear_key(),
        }
        let _ = rebuild();
    }
    res
}

fn restore(name: &str, bytes: Option<Vec<u8>>) {
    let path = certs_dir().join(name);
    match bytes {
        Some(b) => {
            let _ = write_private(&path, &b);
        }
        None => {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub fn clear(kind: &str) -> Result<(), String> {
    match file_name(kind)? {
        KEY_NAME => clear_key(),
        name => {
            let _ = std::fs::remove_file(certs_dir().join(name));
        }
    }
    rebuild()
}

fn write_private(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_garbage_pem() {
        assert!(build_with(Some(b"not a pem".to_vec()), None).is_err());
        assert!(build_with(None, Some(b"not a pem".to_vec())).is_err());
    }

    #[test]
    fn plain_clients_build() {
        assert!(build_with(None, None).is_ok());
    }

    #[test]
    fn unknown_kind_rejected() {
        assert!(file_name("bogus").is_err());
    }
}

#[cfg(test)]
mod split_tests {
    use super::*;

    #[test]
    fn splits_key_from_cert() {
        let pem = "-----BEGIN CERTIFICATE-----\nAAA\n-----END CERTIFICATE-----\n-----BEGIN PRIVATE KEY-----\nBBB\n-----END PRIVATE KEY-----\n";
        let (rest, key) = split_key(pem);
        assert!(rest.contains("CERTIFICATE") && !rest.contains("PRIVATE"));
        assert!(key.contains("BBB") && !key.contains("CERTIFICATE"));
    }
}
