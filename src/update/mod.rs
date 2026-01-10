//! Update checking system for Rustforged.
//!
//! Checks GitHub Releases API for new versions and notifies the user.

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use futures_lite::future;
use semver::Version;
use serde::Deserialize;

/// Current version of the application (from Cargo.toml)
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for checking releases
const GITHUB_REPO: &str = "fitz11/rustforged";

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
}

/// Background task for checking updates
#[derive(Component)]
struct UpdateCheckTask(Task<UpdateCheckResult>);

/// Result of an update check
struct UpdateCheckResult {
    update_available: bool,
    latest_version: Option<String>,
    release_url: Option<String>,
    release_notes: Option<String>,
    error: Option<String>,
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
        Ok(resp) => {
            match resp.into_json::<GitHubRelease>() {
                Ok(release) => {
                    // Skip drafts and prereleases
                    if release.draft || release.prerelease {
                        return UpdateCheckResult {
                            update_available: false,
                            latest_version: None,
                            release_url: None,
                            release_notes: None,
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

                    UpdateCheckResult {
                        update_available,
                        latest_version: Some(version_str.to_string()),
                        release_url: Some(release.html_url),
                        release_notes: release.body,
                        error: None,
                    }
                }
                Err(e) => UpdateCheckResult {
                    update_available: false,
                    latest_version: None,
                    release_url: None,
                    release_notes: None,
                    error: Some(format!("Failed to parse release info: {}", e)),
                },
            }
        }
        Err(ureq::Error::Status(404, _)) => {
            // No releases yet - this is fine
            UpdateCheckResult {
                update_available: false,
                latest_version: None,
                release_url: None,
                release_notes: None,
                error: None,
            }
        }
        Err(e) => UpdateCheckResult {
            update_available: false,
            latest_version: None,
            release_url: None,
            release_notes: None,
            error: Some(format!("Failed to check for updates: {}", e)),
        },
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
            update_state.error = result.error;

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
                ui.add_space(ui.available_width() - 120.0);

                let version = update_state
                    .latest_version
                    .as_deref()
                    .unwrap_or("unknown");

                if ui
                    .colored_label(
                        egui::Color32::from_rgb(255, 165, 0),
                        format!("Update v{} available", version),
                    )
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
pub fn update_dialog_ui(mut contexts: EguiContexts, mut update_state: ResMut<UpdateState>) -> Result {
    if !update_state.show_dialog {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    let mut open = true;
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

            // Buttons
            ui.horizontal(|ui| {
                if let Some(ref url) = update_state.release_url
                    && ui.button("Download").clicked()
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
        });

    if !open {
        update_state.show_dialog = false;
    }

    Ok(())
}

/// Plugin for update checking
pub struct UpdateCheckerPlugin;

impl Plugin for UpdateCheckerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UpdateState>()
            .add_systems(Startup, start_update_check)
            .add_systems(Update, poll_update_check)
            .add_systems(
                EguiPrimaryContextPass,
                (update_indicator_ui, update_dialog_ui),
            );
    }
}
