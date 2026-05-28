use std::fs;
use std::path::PathBuf;

fn main() {
    tauri_build::build();

    // Embed all .toml files from the themes/ directory as static strings so
    // they are available on Android where std::fs cannot read APK assets.
    let themes_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../themes");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let mut entries: Vec<(String, String)> = Vec::new();
    if let Ok(read) = fs::read_dir(&themes_dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") { continue }
            let id = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => continue,
            };
            let content = fs::read_to_string(&path).unwrap_or_default();
            entries.push((id, content));
        }
    }

    // Generate a Rust source file with a static array of (id, toml_content) pairs.
    let mut code = String::from(
        "pub static EMBEDDED_THEMES: &[(&str, &str)] = &[\n"
    );
    for (id, content) in &entries {
        // Escape the content as a raw string using a unique delimiter.
        code.push_str(&format!("    ({:?}, {:?}),\n", id, content));
    }
    code.push_str("];\n");

    fs::write(out_dir.join("embedded_themes.rs"), code).unwrap();

    // Re-run if any theme file changes.
    println!("cargo:rerun-if-changed=../themes");
}
