//! Update UI components: indicator and dialog.

use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy_egui::{egui, EguiContexts};

use super::operations::{download_installer, install_and_restart};
use super::state::{DownloadTask, UpdateState};
use super::CURRENT_VERSION;

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
