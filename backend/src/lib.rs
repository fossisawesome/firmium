// Crate root for firmium-backend. Files stay physically at backend/*.rs /
// backend/commands/*.rs etc (siblings of this src/ dir); #[path] mounts them
// here so no per-file `crate::...` reference needs rewriting.

#[path = "../types.rs"]
pub mod types;
pub use types::{AudioDevice, PlaybackState};
#[path = "../viz_enums.rs"]
pub mod viz_enums;
#[path = "../config.rs"]
pub mod config;
#[path = "../events.rs"]
pub mod events;
#[path = "../errors.rs"]
pub mod errors;
#[path = "../paths.rs"]
pub mod paths;
#[path = "../state.rs"]
pub mod state;
#[path = "../visualizer.rs"]
pub mod visualizer;
#[path = "../audio/mod.rs"]
pub mod audio;
#[path = "../db.rs"]
pub mod db;
#[path = "../queue_state.rs"]
pub mod queue_state;
#[path = "../queue_manager.rs"]
pub mod queue_manager;
#[path = "../commands/mod.rs"]
pub mod commands;
#[path = "../podcasts/mod.rs"]
pub mod podcasts;
#[path = "../init.rs"]
pub mod init;
#[path = "../ipc.rs"]
pub mod ipc;
