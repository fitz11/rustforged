//! Update state resources and task components.

use bevy::prelude::*;
use bevy::tasks::Task;
use std::path::PathBuf;

/// State for the update checker
#[derive(Resource, Default)]
pub struct UpdateState {
    /// Whether we're currently checking for updates
    pub is_checking: bool,
    /// Whether an update is available
    pub update_available: bool,
    /// The latest version available (if any)
    pub latest_version: Option<String>,
    /// URL to the release page
    pub release_url: Option<String>,
    /// Release notes/description
    pub release_notes: Option<String>,
    /// Error message if check failed
    pub error: Option<String>,
    /// Whether to show the update dialog
    pub show_dialog: bool,
    /// Whether the user has dismissed the notification for this session
    pub dismissed: bool,

    // Download state
    /// Direct URL to the installer for current platform
    pub download_url: Option<String>,
    /// Whether we're currently downloading
    pub is_downloading: bool,
    /// Download error message
    pub download_error: Option<String>,
    /// Path to downloaded installer (when complete)
    pub downloaded_path: Option<PathBuf>,
}

/// Background task for checking updates
#[derive(Component)]
pub struct UpdateCheckTask(pub Task<UpdateCheckResult>);

/// Background task for downloading installer
#[derive(Component)]
pub struct DownloadTask(pub Task<DownloadResult>);

/// Result of an update check
pub struct UpdateCheckResult {
    pub update_available: bool,
    pub latest_version: Option<String>,
    pub release_url: Option<String>,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub error: Option<String>,
}

/// Result of downloading an installer
pub struct DownloadResult {
    pub success: bool,
    pub path: Option<PathBuf>,
    pub error: Option<String>,
}

impl UpdateCheckResult {
    pub fn no_update() -> Self {
        Self {
            update_available: false,
            latest_version: None,
            release_url: None,
            release_notes: None,
            download_url: None,
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            update_available: false,
            latest_version: None,
            release_url: None,
            release_notes: None,
            download_url: None,
            error: Some(msg),
        }
    }
}
