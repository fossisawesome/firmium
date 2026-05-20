// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;

fn main() {
    #[cfg(target_os = "linux")]
    {
        if let Some(home_dir) = std::env::var_os("HOME").map(PathBuf::from) {
            let apps_dir = home_dir.join(".local/share/applications");
            let desktop_file = apps_dir.join("firmium.desktop");

            if !desktop_file.exists() {
                let _ = create_dir_all(&apps_dir);

                let exec_path = std::env::var("APPIMAGE").unwrap_or_else(|_| {
                    std::env::current_exe()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_else(|_| "firmium-desktop".to_string())
                });

                let content = format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=Firmium\n\
                     Comment=Subsonic Desktop Music Streamer\n\
                     Exec={}\n\
                     Icon=multimedia-audio-player\n\
                     Terminal=false\n\
                     Categories=Audio;Music;Player;AudioVideo;\n",
                    exec_path
                );

                if let Ok(mut file) = File::create(desktop_file) {
                    let _ = file.write_all(content.as_bytes());
                }
            }
        }
    }

    firmium_desktop_lib::run();
}