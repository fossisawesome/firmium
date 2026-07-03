// keyring is only available on desktop OSes.
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos", target_os = "freebsd"))]
use keyring::Entry;

// ============================================================================
// KEYRING / CREDENTIALS MANAGEMENT
// ============================================================================

/// Default keyring service name. Used when no explicit service is provided
/// (backwards compatibility with single-server setups).
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos", target_os = "freebsd"))]
const DEFAULT_SERVICE: &str = "firmium-desktop";

/// Resolves the keyring service name. For multi-server support, the frontend
/// can pass the server URL as the service; if empty/missing, falls back to
/// the default so existing single-server keyring entries keep working.
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos", target_os = "freebsd"))]
fn resolve_service(service: Option<&str>) -> &str {
    match service {
        Some(s) if !s.is_empty() => s,
        _ => DEFAULT_SERVICE,
    }
}

/// Save a password to the OS keyring.
pub fn save_password(_service: Option<&str>, _user: &str, _pass: &str) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos", target_os = "freebsd"))]
    {
        let entry = Entry::new(resolve_service(_service), _user).map_err(|e| e.to_string())?;
        entry.set_password(_pass).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Retrieve a password from the OS keyring.
pub fn get_password(_service: Option<&str>, _user: &str) -> Result<String, String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos", target_os = "freebsd"))]
    {
        let entry = Entry::new(resolve_service(_service), _user).map_err(|e| e.to_string())?;
        return entry.get_password().map_err(|e| e.to_string());
    }
    #[allow(unreachable_code)]
    Err("Keyring not available on this platform".to_string())
}

/// Delete a password from the OS keyring.
pub fn delete_password(_service: Option<&str>, _user: &str) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos", target_os = "freebsd"))]
    {
        let entry = Entry::new(resolve_service(_service), _user).map_err(|e| e.to_string())?;
        entry.delete_credential().map_err(|e| e.to_string())?;
    }
    Ok(())
}
