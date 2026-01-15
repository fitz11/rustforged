//! Update checking and auto-update system for Rustforged.
//!
//! Checks GitHub Releases API for new versions, downloads installers,
//! and launches the installer when the user is ready.

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use futures_lite::future;
use semver::Version;
use serde::Deserialize;
use std::path::PathBuf;

/// Current version of the application (from Cargo.toml)
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for checking releases
const GITHUB_REPO: &str = "fitz11/rustforged";

/// GitHub Release asset structure
#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// GitHub Releases API response structure
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[allow(dead_code)]
    name: Option<String>,
    body: Option<String>,
    prerelease: bool,
    draft: bool,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
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

/// Find the installer URL for the current platform
fn get_installer_url(assets: &[GitHubAsset]) -> Option<String> {
    #[cfg(target_os = "windows")]
    let suffix = if cfg!(target_arch = "aarch64") {
        "arm64.msi"
    } else {
        "x64.msi"
    };

    #[cfg(target_os = "macos")]
    let suffix = "aarch64.dmg";

    #[cfg(target_os = "linux")]
    let suffix = ""; // Linux users build from source

    if suffix.is_empty() {
        return None;
    }

    assets
        .iter()
        .find(|a| a.name.ends_with(suffix))
        .map(|a| a.browser_download_url.clone())
}

/// Check for updates against GitHub Releases API
fn check_github_releases() -> UpdateCheckResult {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = ureq::get(&url)
        .set("User-Agent", "rustforged-update-checker")
        .set("Accept", "application/vnd.github.v3+json")
        .call();

    match response {
        Ok(resp) => match resp.into_json::<GitHubRelease>() {
            Ok(release) => {
                // Skip drafts and prereleases
                if release.draft || release.prerelease {
                    return UpdateCheckResult {
                        update_available: false,
                        latest_version: None,
                        release_url: None,
                        release_notes: None,
                        download_url: None,
                        error: None,
                    };
                }

                // Parse version (remove 'v' prefix if present)
                let version_str = release.tag_name.trim_start_matches('v');

                let update_available = match (
                    Version::parse(version_str),
                    Version::parse(CURRENT_VERSION),
                ) {
                    (Ok(latest), Ok(current)) => latest > current,
                    _ => false, // If we can't parse versions, assume no update
                };

                let download_url = if update_available {
                    get_installer_url(&release.assets)
                } else {
                    None
                };

                UpdateCheckResult {
                    update_available,
                    latest_version: Some(version_str.to_string()),
                    release_url: Some(release.html_url),
                    release_notes: release.body,
                    download_url,
                    error: None,
                }
            }
            Err(e) => UpdateCheckResult {
                update_available: false,
                latest_version: None,
                release_url: None,
                release_notes: None,
                download_url: None,
                error: Some(format!("Failed to parse release info: {}", e)),
            },
        },
        Err(ureq::Error::Status(404, _)) => {
            // No releases yet - this is fine
            UpdateCheckResult {
                update_available: false,
                latest_version: None,
                release_url: None,
                release_notes: None,
                download_url: None,
                error: None,
            }
        }
        Err(e) => UpdateCheckResult {
            update_available: false,
            latest_version: None,
            release_url: None,
            release_notes: None,
            download_url: None,
            error: Some(format!("Failed to check for updates: {}", e)),
        },
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
    let task = task_pool.spawn(async move { check_github_releases() });

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
