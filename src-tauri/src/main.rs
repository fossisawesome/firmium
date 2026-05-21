use keyring::Entry;
use std::fs::{self, File};
use std::io::Write;
use tauri::Manager;
use sysinfo::System;

#[derive(serde::Serialize)]
struct SystemInfo {
    cpu: String,
    gpu: String,
    distro: String,
    version: String,
    package_manager: String,
}

#[tauri::command]
fn save_password(service: &str, user: &str, pass: &str) -> Result<(), String> {
    let entry = Entry::new(service, user).map_err(|e| e.to_string())?;
    entry.set_password(pass).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_password(service: &str, user: &str) -> Result<String, String> {
    let entry = Entry::new(service, user).map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_password(service: &str, user: &str) -> Result<(), String> {
    let entry = Entry::new(service, user).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn cache_cover(app_handle: tauri::AppHandle, id: String, server_url: String) -> Result<String, String> {
    let mut cache_path = app_handle.path().app_cache_dir().map_err(|e| e.to_string())?;
    cache_path.push("covers");
    fs::create_dir_all(&cache_path).map_err(|e| e.to_string())?;
    cache_path.push(format!("{}.img", id));

    if cache_path.exists() {
        return Ok(cache_path.to_string_lossy().into_owned());
    }

    let response = reqwest::get(&server_url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err("Failed to download cover from server".into());
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let mut file = File::create(&cache_path).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;

    Ok(cache_path.to_string_lossy().into_owned())
}

#[tauri::command]
fn get_machine_info(app_handle: tauri::AppHandle) -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_else(|| "Unknown CPU".to_string());
    let distro = System::name().unwrap_or_else(|| "Unknown Linux".to_string());
    let version = app_handle.package_info().version.to_string();

    let mut package_manager = "Native package manager".to_string();
    if std::env::var("FLATPAK_ID").is_ok() {
        package_manager = "Flatpak".to_string();
    } else if std::env::var("SNAP").is_ok() {
        package_manager = "Snap".to_string();
    } else if std::env::var("APPIMAGE").is_ok() {
        package_manager = "AppImage".to_string();
    }

    let gpu = std::process::Command::new("sh")
        .arg("-c")
        .arg("lspci | grep -E 'VGA|3D' | cut -d ':' -f3 | sed 's/^[ \t]*//'")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Unknown GPU".to_string());

    let final_gpu = if gpu.is_empty() { "Standard Graphics Adapter".to_string() } else { gpu };

    SystemInfo { cpu, gpu: final_gpu, distro, version, package_manager }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            save_password,
            get_password,
            delete_password,
            cache_cover,
            get_machine_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}