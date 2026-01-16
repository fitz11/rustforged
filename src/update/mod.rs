//! Update checking and auto-update system for Rustforged.
//!
//! Fetches a release manifest JSON file to check for new versions, downloads installers,
//! and launches the installer when the user is ready.
//!
//! The manifest format is generic and can be hosted anywhere (GitHub Pages, S3, any CDN).
//! To migrate away from GitHub, simply change `MANIFEST_URL` in `manifest.rs` to point
//! to your new host.
//!
//! ## Module Structure
//!
//! - [`manifest`] - Release manifest structure and version comparison
//! - [`state`] - Update state resources and async task components
//! - [`operations`] - Core update operations (check, download, install)
//! - [`systems`] - Bevy systems for async task management
//! - [`ui`] - UI components (indicator and dialog)
//!
//! ## Key Types
//!
//! - [`UpdateState`] - Main resource tracking update status
//! - `ReleaseManifest` - Parsed release manifest from remote
//!
//! ## Systems
//!
//! - [`start_update_check`] - Startup system to begin update check
//! - [`poll_update_check`] - Polls async update check task
//! - [`poll_download_task`] - Polls async download task
//! - [`update_indicator_ui`] - Shows update indicator in toolbar
//! - [`update_dialog_ui`] - Shows update dialog window

mod manifest;
mod operations;
mod state;
mod systems;
mod ui;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

// Re-exports
pub use state::UpdateState;
pub use systems::{poll_download_task, poll_update_check, start_update_check};
pub use ui::{update_dialog_ui, update_indicator_ui};

/// Current version of the application (from Cargo.toml)
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Plugin for update checking
pub struct UpdateCheckerPlugin;

impl Plugin for UpdateCheckerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UpdateState>()
            .add_systems(Startup, start_update_check)
            .add_systems(Update, (poll_update_check, poll_download_task))
            .add_systems(
                EguiPrimaryContextPass,
                (update_indicator_ui, update_dialog_ui),
            );
    }
}
