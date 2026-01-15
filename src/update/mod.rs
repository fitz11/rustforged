//! Update checking and auto-update system for Rustforged.
//!
//! Fetches a release manifest JSON file to check for new versions, downloads installers,
//! and launches the installer when the user is ready.
//!
//! The manifest format is generic and can be hosted anywhere (GitHub Pages, S3, any CDN).
//! To migrate away from GitHub, simply change `MANIFEST_URL` to point to your new host.

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use futures_lite::future;
use semver::Version;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Current version of the application (from Cargo.toml)
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// URL to the release manifest file.
/// Change this constant to migrate away from GitHub hosting.
const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/fitz11/rustforged/main/releases/latest.json";

/// Release manifest structure - can be hosted anywhere.
///
/// Example JSON:
/// ```json
/// {
///   "version": "1.2.3",
///   "release_url": "https://github.com/fitz11/rustforged/releases/v1.2.3",
///   "release_notes": "Bug fixes and improvements...",
///   "assets": {
///     "windows-x64": "https://example.com/rustforged-1.2.3-x64.msi",
///     "windows-arm64": "https://example.com/rustforged-1.2.3-arm64.msi",
///     "macos-aarch64": "https://example.com/rustforged-1.2.3-aarch64.dmg"
///   }
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct ReleaseManifest {
    /// Latest version string (semver format, e.g., "1.2.3")
    pub version: String,
    /// URL to the release page for manual download
    pub release_url: String,
    /// Release notes/changelog (optional)
    pub release_notes: Option<String>,
    /// Platform-specific download URLs (key: platform, value: URL)
    #[serde(default)]
    pub assets: HashMap<String, String>,
}

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
struct UpdateCheckTask(Task<UpdateCheckResult>);

/// Background task for downloading installer
#[derive(Component)]
struct DownloadTask(Task<DownloadResult>);

/// Result of an update check
struct UpdateCheckResult {
    update_available: bool,
    latest_version: Option<String>,
    release_url: Option<String>,
    release_notes: Option<String>,
    download_url: Option<String>,
    error: Option<String>,
}

/// Result of downloading an installer
struct DownloadResult {
    success: bool,
    path: Option<PathBuf>,
    error: Option<String>,
}

impl UpdateCheckResult {
    fn no_update() -> Self {
        Self {
            update_available: false,
            latest_version: None,
            release_url: None,
            release_notes: None,
            download_url: None,
            error: None,
        }
    }

    fn error(msg: String) -> Self {
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

/// Compare versions, returns true if `latest` is newer than `current`
fn is_newer_version(latest: &str, current: &str) -> bool {
    match (Version::parse(latest), Version::parse(current)) {
        (Ok(latest_v), Ok(current_v)) => latest_v > current_v,
        _ => false,
    }
}

/// Get the platform key for the current build target
fn current_platform_key() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-x64"
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        "windows-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-aarch64"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x64"
    }
    // Fallback for other configurations (e.g., macOS x86_64)
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64")
    )))]
    {
        "unknown"
    }
}

/// Get download URL for current platform from assets map
fn get_platform_asset(assets: &HashMap<String, String>) -> Option<String> {
    assets.get(current_platform_key()).cloned()
}

/// Check for updates by fetching the release manifest
fn check_for_updates() -> UpdateCheckResult {
    let response = ureq::get(MANIFEST_URL)
        .set("User-Agent", "rustforged-update-checker")
        .call();

    match response {
        Ok(resp) => match resp.into_json::<ReleaseManifest>() {
            Ok(manifest) => {
                let update_available = is_newer_version(&manifest.version, CURRENT_VERSION);
                let download_url = if update_available {
                    get_platform_asset(&manifest.assets)
                } else {
                    None
                };

                UpdateCheckResult {
                    update_available,
                    latest_version: Some(manifest.version),
                    release_url: Some(manifest.release_url),
                    release_notes: manifest.release_notes,
                    download_url,
                    error: None,
                }
            }
            Err(e) => UpdateCheckResult::error(format!("Failed to parse manifest: {}", e)),
        },
        Err(ureq::Error::Status(404, _)) => {
            // No manifest yet - this is fine
            UpdateCheckResult::no_update()
        }
        Err(e) => UpdateCheckResult::error(format!("Failed to check for updates: {}", e)),
    }
}

/// Download the installer to a temp directory
fn download_installer(url: String, version: String) -> DownloadResult {
    let temp_dir = std::env::temp_dir();

    #[cfg(target_os = "windows")]
    let filename = format!("rustforged-{}.msi", version);

    #[cfg(target_os = "macos")]
    let filename = format!("rustforged-{}.dmg", version);

    #[cfg(target_os = "linux")]
    let filename = format!("rustforged-{}.tar.gz", version);

    let path = temp_dir.join(&filename);

    match ureq::get(&url)
        .set("User-Agent", "rustforged-updater")
        .call()
    {
        Ok(response) => {
            let mut file = match std::fs::File::create(&path) {
                Ok(f) => f,
                Err(e) => {
                    return DownloadResult {
                        success: false,
                        path: None,
                        error: Some(format!("Failed to create file: {}", e)),
                    }
                }
            };

            if let Err(e) = std::io::copy(&mut response.into_reader(), &mut file) {
                // Clean up partial file
                let _ = std::fs::remove_file(&path);
                return DownloadResult {
                    success: false,
                    path: None,
                    error: Some(format!("Download failed: {}", e)),
                };
            }

            DownloadResult {
                success: true,
                path: Some(path),
                error: None,
            }
        }
        Err(e) => DownloadResult {
            success: false,
            path: None,
            error: Some(format!("Download failed: {}", e)),
        },
    }
}

/// Launch the installer and exit the app
#[allow(unused_variables)]
fn install_and_restart(installer_path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Launch MSI installer in passive mode and exit
        std::process::Command::new("msiexec")
            .args(["/i", &installer_path.to_string_lossy(), "/passive"])
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;

        std::process::exit(0);
    }

    #[cfg(target_os = "macos")]
    {
        // Open DMG - user will drag to Applications
        std::process::Command::new("open")
            .arg(installer_path)
            .spawn()
            .map_err(|e| format!("Failed to open DMG: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        // Linux users should build from source
        Err("Auto-update not supported on Linux. Please build from source.".to_string())
    }
}

/// System to start the update check on startup
fn start_update_check(mut commands: Commands, mut update_state: ResMut<UpdateState>) {
    update_state.is_checking = true;

    let task_pool = AsyncComputeTaskPool::get();
    let task = task_pool.spawn(async move { check_for_updates() });

    commands.spawn(UpdateCheckTask(task));
}

/// System to poll the update check task
fn poll_update_check(
    mut commands: Commands,
    mut update_state: ResMut<UpdateState>,
    mut tasks: Query<(Entity, &mut UpdateCheckTask)>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            update_state.is_checking = false;
            update_state.update_available = result.update_available;
            update_state.latest_version = result.latest_version;
            update_state.release_url = result.release_url;
            update_state.release_notes = result.release_notes;
            update_state.download_url = result.download_url;
            update_state.error = result.error;

            commands.entity(entity).despawn();
        }
    }
}

/// System to poll the download task
fn poll_download_task(
    mut commands: Commands,
    mut update_state: ResMut<UpdateState>,
    mut tasks: Query<(Entity, &mut DownloadTask)>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            update_state.is_downloading = false;

            if result.success {
                update_state.downloaded_path = result.path;
                update_state.download_error = None;
            } else {
                update_state.download_error = result.error;
            }

            commands.entity(entity).despawn();
        }
    }
}

/// UI system to show update indicator in toolbar
pub fn update_indicator_ui(
    mut contexts: EguiContexts,
    mut update_state: ResMut<UpdateState>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    // Only show if update available and not dismissed
    if !update_state.update_available || update_state.dismissed {
        return Ok(());
    }

    egui::TopBottomPanel::top("update_indicator")
        .frame(egui::Frame::NONE)
        .show_separator_line(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() - 150.0);

                let version = update_state
                    .latest_version
                    .as_deref()
                    .unwrap_or("unknown");

                let label_text = if update_state.is_downloading {
                    "Downloading update...".to_string()
                } else if update_state.downloaded_path.is_some() {
                    "Update ready to install".to_string()
                } else {
                    format!("Update v{} available", version)
                };

                if ui
                    .colored_label(egui::Color32::from_rgb(255, 165, 0), label_text)
                    .on_hover_text("Click to view release details")
                    .clicked()
                {
                    update_state.show_dialog = true;
                }
            });
        });

    Ok(())
}

/// UI system to show the update dialog
#[allow(clippy::too_many_lines, unused_mut)]
pub fn update_dialog_ui(
    mut contexts: EguiContexts,
    mut update_state: ResMut<UpdateState>,
    mut commands: Commands,
) -> Result {
    if !update_state.show_dialog {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    let mut open = true;
    let mut start_download = false;
    let mut start_install = false;

    egui::Window::new("Update Available")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_min_width(400.0);

            // Version info
            ui.horizontal(|ui| {
                ui.label("Current version:");
                ui.strong(CURRENT_VERSION);
            });

            if let Some(ref version) = update_state.latest_version {
                ui.horizontal(|ui| {
                    ui.label("Latest version:");
                    ui.strong(version);
                });
            }

            ui.add_space(10.0);

            // Release notes
            if let Some(ref notes) = update_state.release_notes {
                ui.label("Release notes:");
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.label(notes);
                    });
                ui.add_space(10.0);
            }

            // Show different UI based on state
            if update_state.is_downloading {
                // Downloading state
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Downloading update...");
                });
            } else if let Some(path) = update_state.downloaded_path.clone() {
                // Download complete - ready to install
                ui.colored_label(
                    egui::Color32::from_rgb(0, 200, 0),
                    "Download complete! Ready to install.",
                );
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    #[cfg(target_os = "windows")]
                    {
                        if ui.button("Install & Restart").clicked() {
                            start_install = true;
                        }
                    }

                    #[cfg(target_os = "macos")]
                    {
                        if ui.button("Open Installer").clicked() {
                            start_install = true;
                        }
                        ui.label("(Drag to Applications, then restart)");
                    }

                    #[cfg(target_os = "linux")]
                    {
                        ui.label("Downloaded to:");
                        ui.monospace(path.to_string_lossy().to_string());
                    }

                    if ui.button("Later").clicked() {
                        update_state.show_dialog = false;
                    }
                });
            } else if let Some(ref error) = update_state.download_error {
                // Download error
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), error);
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    if update_state.download_url.is_some() && ui.button("Try Again").clicked() {
                        start_download = true;
                    }

                    if let Some(ref url) = update_state.release_url
                        && ui.button("Download Manually").clicked()
                    {
                        let _ = open::that(url);
                    }

                    if ui.button("Close").clicked() {
                        update_state.show_dialog = false;
                    }
                });
            } else {
                // Initial state - show download options
                ui.horizontal(|ui| {
                    // Show Download button only if we have a URL for this platform
                    if update_state.download_url.is_some() {
                        if ui.button("Download & Install").clicked() {
                            start_download = true;
                        }
                    } else {
                        // No installer for this platform (Linux)
                        #[cfg(target_os = "linux")]
                        {
                            ui.label("Please build from source to update.");
                        }
                    }

                    // Always show manual download option
                    if let Some(ref url) = update_state.release_url
                        && ui.button("View Release").clicked()
                    {
                        let _ = open::that(url);
                    }

                    if ui.button("Later").clicked() {
                        update_state.show_dialog = false;
                    }

                    if ui.button("Dismiss").clicked() {
                        update_state.show_dialog = false;
                        update_state.dismissed = true;
                    }
                });
            }
        });

    if !open {
        update_state.show_dialog = false;
    }

    // Handle actions after UI rendering
    if start_download
        && let (Some(url), Some(version)) = (
            update_state.download_url.clone(),
            update_state.latest_version.clone(),
        )
    {
        update_state.is_downloading = true;
        update_state.download_error = None;

        let task_pool = AsyncComputeTaskPool::get();
        let task = task_pool.spawn(async move { download_installer(url, version) });

        commands.spawn(DownloadTask(task));
    }

    if start_install
        && let Some(ref path) = update_state.downloaded_path
        && let Err(e) = install_and_restart(path)
    {
        update_state.download_error = Some(e);
    }

    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // Version comparison tests
    #[test]
    fn test_is_newer_version_true() {
        assert!(is_newer_version("1.1.0", "1.0.0"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(is_newer_version("1.0.1", "1.0.0"));
    }

    #[test]
    fn test_is_newer_version_false() {
        assert!(!is_newer_version("1.0.0", "1.0.0")); // Same version
        assert!(!is_newer_version("1.0.0", "1.1.0")); // Older
        assert!(!is_newer_version("0.9.0", "1.0.0")); // Much older
    }

    #[test]
    fn test_is_newer_version_invalid() {
        assert!(!is_newer_version("invalid", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "invalid"));
        assert!(!is_newer_version("", "1.0.0"));
    }

    // Manifest parsing tests
    #[test]
    fn test_manifest_parsing_full() {
        let json = r#"{
            "version": "1.2.3",
            "release_url": "https://example.com/releases/v1.2.3",
            "release_notes": "New features",
            "assets": {
                "windows-x64": "https://example.com/win.msi",
                "macos-aarch64": "https://example.com/mac.dmg"
            }
        }"#;

        let manifest: ReleaseManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "1.2.3");
        assert_eq!(manifest.release_url, "https://example.com/releases/v1.2.3");
        assert_eq!(manifest.release_notes, Some("New features".to_string()));
        assert_eq!(manifest.assets.len(), 2);
    }

    #[test]
    fn test_manifest_parsing_minimal() {
        let json = r#"{
            "version": "1.0.0",
            "release_url": "https://example.com"
        }"#;

        let manifest: ReleaseManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.release_notes.is_none());
        assert!(manifest.assets.is_empty());
    }

    // Platform asset selection tests
    #[test]
    fn test_get_platform_asset_found() {
        let mut assets = HashMap::new();
        assets.insert(
            current_platform_key().to_string(),
            "https://example.com/installer".to_string(),
        );

        let result = get_platform_asset(&assets);
        assert_eq!(result, Some("https://example.com/installer".to_string()));
    }

    #[test]
    fn test_get_platform_asset_missing() {
        let assets = HashMap::new();
        let result = get_platform_asset(&assets);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_platform_asset_wrong_platform() {
        let mut assets = HashMap::new();
        // Insert a platform that doesn't match current
        assets.insert(
            "nonexistent-platform".to_string(),
            "https://example.com/installer".to_string(),
        );

        let result = get_platform_asset(&assets);
        assert!(result.is_none());
    }

    // UpdateCheckResult helper tests
    #[test]
    fn test_update_check_result_no_update() {
        let result = UpdateCheckResult::no_update();
        assert!(!result.update_available);
        assert!(result.latest_version.is_none());
        assert!(result.release_url.is_none());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_update_check_result_error() {
        let result = UpdateCheckResult::error("Network error".to_string());
        assert!(!result.update_available);
        assert_eq!(result.error, Some("Network error".to_string()));
    }

    // Platform key tests
    #[test]
    fn test_current_platform_key_not_empty() {
        let key = current_platform_key();
        assert!(!key.is_empty());
        // Should be one of the known platforms or "unknown"
        let valid_keys = [
            "windows-x64",
            "windows-arm64",
            "macos-aarch64",
            "linux-x64",
            "unknown",
        ];
        assert!(valid_keys.contains(&key));
    }
}
