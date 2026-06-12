// keyring is only available on desktop OSes.
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
use keyring::Entry;

// ============================================================================
// KEYRING / CREDENTIALS MANAGEMENT
// ============================================================================

/// Keyring service name for all Firmium credentials. Pinned here rather than
/// accepted from the frontend so the IPC surface can't be used to read/write
/// arbitrary keyring entries.
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
const SERVICE: &str = "firmium-desktop";

/// Save a password to the OS keyring.
#[tauri::command]
pub fn save_password(_user: &str, _pass: &str) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        let entry = Entry::new(SERVICE, _user).map_err(|e| e.to_string())?;
        entry.set_password(_pass).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Retrieve a password from the OS keyring.
#[tauri::command]
pub fn get_password(_user: &str) -> Result<String, String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        let entry = Entry::new(SERVICE, _user).map_err(|e| e.to_string())?;
        return entry.get_password().map_err(|e| e.to_string());
    }
    #[allow(unreachable_code)]
    Err("Keyring not available on this platform".to_string())
}

/// Delete a password from the OS keyring.
#[tauri::command]
pub fn delete_password(_user: &str) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        let entry = Entry::new(SERVICE, _user).map_err(|e| e.to_string())?;
        entry.delete_credential().map_err(|e| e.to_string())?;
    }
    Ok(())
}
