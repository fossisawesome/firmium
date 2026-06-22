/// Return the app version string from Cargo.toml at compile time.
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
