use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use std::path::{Path, PathBuf};

use crate::assets::{AssetLibrary, RetargetAssetRequest};
use crate::config::{AppConfig, ConfigResetNotification, MissingMapWarning, SaveConfigRequest};
use crate::map::{
    AsyncMapOperation, LoadMapRequest, MapAssetReport, MapLoadError, MapSaveError, SaveConflict,
    SaveConflictDialog, SaveConflictKind, SaveMapRequest,
};

use super::map_actions::{UnsavedChangesPrompt, sanitize_map_name};

#[derive(Resource, Default)]
pub struct FileMenuState {
    pub show_save_name_dialog: bool,
    pub save_filename: String,
}

/// Renders the dialog windows for file operations (triggered from asset_browser menu)
pub fn file_menu_ui(
    mut contexts: EguiContexts,
    mut menu_state: ResMut<FileMenuState>,
    mut prompt: ResMut<UnsavedChangesPrompt>,
    mut save_events: MessageWriter<SaveMapRequest>,
    mut load_error: ResMut<MapLoadError>,
    library: Res<AssetLibrary>,
) -> Result {
    // Save dialog for filename
    if menu_state.show_save_name_dialog {
        // Use library's maps directory
        let maps_dir = library.library_path.join("maps");

        egui::Window::new("Save Map")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                // Show destination info
                ui.label(
                    egui::RichText::new(format!("Saving to: {}/", maps_dir.display()))
                        .small()
                        .weak(),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Map name:");
                    ui.text_edit_singleline(&mut menu_state.save_filename);
                });
                // A name of only spaces or symbols would make a hidden ".json" file
                let filename = sanitize_map_name(&menu_state.save_filename);
                let name_ok = filename.chars().any(char::is_alphanumeric);
                if !name_ok {
                    ui.label(egui::RichText::new("Enter a name with letters or digits").weak());
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(name_ok, egui::Button::new("Save")).clicked() {
                        let path = maps_dir.join(format!("{}.json", filename));
                        let request = SaveMapRequest::new(path);
                        prompt.wait_for(&request);
                        save_events.write(request);
                        menu_state.show_save_name_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        menu_state.show_save_name_dialog = false;
                        // Backed out of the save, so don't go on to replace or close the map
                        prompt.after_save = None;
                    }
                });
            });
    }

    // Load error dialog
    if let Some(error) = &load_error.message {
        let mut dismiss = false;
        egui::Window::new("Load Error")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.colored_label(egui::Color32::RED, error);
                    });
                ui.add_space(8.0);
                if ui.button("OK").clicked() {
                    dismiss = true;
                }
            });
        if dismiss {
            load_error.message = None;
        }
    }

    Ok(())
}

/// Renders the missing map warning dialog (shown at startup if last map doesn't exist)
pub fn missing_map_warning_ui(
    mut contexts: EguiContexts,
    mut warning: ResMut<MissingMapWarning>,
    mut config: ResMut<AppConfig>,
    mut save_events: MessageWriter<SaveConfigRequest>,
) -> Result {
    if !warning.show {
        return Ok(());
    }

    egui::Window::new("Map Not Found")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.label("The last opened map file no longer exists:");

            if let Some(ref path) = warning.path {
                ui.add_space(5.0);
                let path_str = path.to_string_lossy();
                let display_path = if path_str.len() > 50 {
                    format!("...{}", &path_str[path_str.len() - 47..])
                } else {
                    path_str.to_string()
                };
                ui.label(egui::RichText::new(display_path).weak())
                    .on_hover_text(&*path_str);
                ui.add_space(10.0);
            }

            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    warning.show = false;
                }

                if ui.button("Clear from history").clicked() {
                    config.data.last_map_path = None;
                    config.dirty = true;
                    save_events.write(SaveConfigRequest);
                    warning.show = false;
                }
            });
        });

    Ok(())
}

/// Modal dialog shown while save/load operations are in progress
pub fn async_operation_modal_ui(
    mut contexts: EguiContexts,
    async_op: Res<AsyncMapOperation>,
) -> Result {
    if !async_op.is_busy() {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    // Semi-transparent overlay to block interaction
    egui::Area::new(egui::Id::new("async_op_overlay"))
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            // Use viewport inner_rect for full-screen overlay
            let rect = ctx.input(|i| {
                i.viewport().inner_rect.unwrap_or(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(1920.0, 1080.0),
                ))
            });
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(100));
        });

    // Modal dialog
    egui::Window::new("Please Wait")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(200.0);

            // Show operation description
            if let Some(ref desc) = async_op.operation_description {
                ui.heading(desc);
            } else if async_op.is_saving {
                ui.heading("Saving...");
            } else {
                ui.heading("Loading...");
            }

            ui.add_space(10.0);
            ui.spinner();
        });

    Ok(())
}

/// Renders the save error dialog when a save operation fails
pub fn save_error_dialog_ui(
    mut contexts: EguiContexts,
    mut save_error: ResMut<MapSaveError>,
    mut menu_state: ResMut<FileMenuState>,
) -> Result {
    let Some(error_msg) = save_error.message.clone() else {
        return Ok(());
    };

    let mut should_clear = false;
    let mut should_show_save_as = false;

    egui::Window::new("Save Failed")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.colored_label(egui::Color32::RED, "Failed to save map:");
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    ui.label(&error_msg);
                });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    should_clear = true;
                }

                if ui.button("Save As...").clicked() {
                    should_show_save_as = true;
                    should_clear = true;
                }
            });
        });

    if should_clear {
        save_error.message = None;
    }
    if should_show_save_as {
        menu_state.show_save_name_dialog = true;
    }

    Ok(())
}

/// Notification dialog shown when config was reset to defaults
pub fn config_reset_notification_ui(
    mut contexts: EguiContexts,
    mut notification: ResMut<ConfigResetNotification>,
) -> Result {
    if !notification.show {
        return Ok(());
    }

    egui::Window::new("Configuration Reset")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.label("Your configuration file could not be loaded.");
            ui.add_space(4.0);

            if let Some(ref reason) = notification.reason {
                ui.label(egui::RichText::new(reason).weak().small());
                ui.add_space(4.0);
            }

            ui.label("Default settings have been applied.");
            ui.add_space(8.0);

            if ui.button("OK").clicked() {
                notification.show = false;
                notification.reason = None;
            }
        });

    Ok(())
}

/// A free file name next to `path` for saving a copy, e.g. "cave (LAPTOP copy).json".
fn copy_path(path: &Path, machine: Option<&str>) -> PathBuf {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("map");
    let label = machine.map_or("copy".to_string(), |m| format!("{} copy", m));
    (1..)
        .map(|n| {
            let suffix = if n == 1 {
                label.clone()
            } else {
                format!("{} {}", label, n)
            };
            path.with_file_name(format!("{} ({}).json", stem, suffix))
        })
        .find(|candidate| !candidate.exists())
        .expect("unbounded range always finds a free name")
}

/// " (saved by DESKTOP, 2026-10-12 14:02)", from whatever the conflict knows.
fn describe_disk_version(conflict: &SaveConflict) -> String {
    let when = conflict.modified.map(|t| {
        chrono::DateTime::<chrono::Local>::from(t)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    });
    match (&conflict.saved_by, when) {
        (Some(who), Some(when)) => format!(" (saved by {}, {})", who, when),
        (Some(who), None) => format!(" (saved by {})", who),
        (None, Some(when)) => format!(" (saved {})", when),
        (None, None) => String::new(),
    }
}

/// Renders the dialog for a save that stopped because the file on disk isn't the version
/// the map was opened from, e.g. it was saved on another machine and synced here.
pub fn save_conflict_dialog_ui(
    mut contexts: EguiContexts,
    mut dialog: ResMut<SaveConflictDialog>,
    mut prompt: ResMut<UnsavedChangesPrompt>,
    mut save_events: MessageWriter<SaveMapRequest>,
    mut load_events: MessageWriter<LoadMapRequest>,
) -> Result {
    let Some(conflict) = dialog.conflict.clone() else {
        return Ok(());
    };

    let name = conflict
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("map");
    let version = describe_disk_version(&conflict);
    let copy = copy_path(&conflict.path, crate::fsutil::machine_name().as_deref());
    let copy_name = copy
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("copy")
        .to_string();
    let mut close = false;

    egui::Window::new("Map Changed on Disk")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            match conflict.kind {
                SaveConflictKind::Changed => {
                    ui.label(format!(
                        "\"{}\" was changed on disk after you opened it{}.",
                        name, version
                    ));
                    ui.label("Saving now would overwrite those changes.");
                }
                SaveConflictKind::Exists => {
                    ui.label(format!(
                        "A map named \"{}\" already exists{}.",
                        name, version
                    ));
                }
                SaveConflictKind::Deleted => {
                    ui.label(format!(
                        "\"{}\" was moved or deleted on disk after you opened it.",
                        name
                    ));
                }
            }
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if conflict.kind == SaveConflictKind::Changed
                    && ui
                        .button("Reload from Disk")
                        .on_hover_text("Discard your unsaved changes and open the version on disk")
                        .clicked()
                {
                    load_events.write(LoadMapRequest {
                        path: conflict.path.clone(),
                    });
                    // Reloading replaces the map, so whatever waited for this save is off
                    prompt.after_save = None;
                    close = true;
                }

                let overwrite_label = if conflict.kind == SaveConflictKind::Deleted {
                    "Save Anyway"
                } else {
                    "Overwrite"
                };
                if ui.button(overwrite_label).clicked() {
                    let request = SaveMapRequest::overwrite(conflict.path.clone());
                    prompt.wait_for(&request);
                    save_events.write(request);
                    close = true;
                }

                if ui
                    .button("Save as Copy")
                    .on_hover_text(format!("Save your version as \"{}\"", copy_name))
                    .clicked()
                {
                    let request = SaveMapRequest::new(copy.clone());
                    prompt.wait_for(&request);
                    save_events.write(request);
                    close = true;
                }

                if ui.button("Cancel").clicked() {
                    prompt.after_save = None;
                    close = true;
                }
            });
        });

    if close {
        dialog.conflict = None;
    }

    Ok(())
}

/// Renders the report shown after opening a map whose asset references needed attention
pub fn map_asset_report_ui(
    mut contexts: EguiContexts,
    mut report: ResMut<MapAssetReport>,
    mut retarget_events: MessageWriter<RetargetAssetRequest>,
) -> Result {
    if !report.show {
        return Ok(());
    }
    let mut relink: Vec<(String, String)> = Vec::new();

    let mut dismiss = false;
    egui::Window::new("Map Assets")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.label(
                egui::RichText::new(format!("Map: {}", report.map_name))
                    .weak()
                    .small(),
            );
            ui.add_space(4.0);

            if !report.relinked.is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "Found {} moved or renamed asset(s):",
                        report.relinked.len()
                    ))
                    .strong(),
                );
                egui::ScrollArea::vertical()
                    .id_salt("relinked_assets")
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for (old, new) in &report.relinked {
                            ui.label(egui::RichText::new(format!("  {} -> {}", old, new)).weak());
                        }
                    });
                ui.label(
                    egui::RichText::new("Save the map to keep the new paths.")
                        .weak()
                        .small(),
                );
                ui.add_space(8.0);
            }

            if !report.missing.is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "{} asset(s) aren't in this library:",
                        report.missing.len()
                    ))
                    .strong(),
                );
                egui::ScrollArea::vertical()
                    .id_salt("missing_assets")
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for (asset_path, candidate) in &report.missing {
                            ui.label(
                                egui::RichText::new(format!("  - {}", asset_path))
                                    .color(egui::Color32::from_rgb(200, 100, 100)),
                            );
                            if let Some(candidate) = candidate {
                                ui.horizontal(|ui| {
                                    ui.add_space(16.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} has the same name, but different contents",
                                            candidate
                                        ))
                                        .weak()
                                        .small(),
                                    );
                                    if ui
                                        .small_button("Use it")
                                        .on_hover_text(
                                            "Point every item using the missing asset at this file",
                                        )
                                        .clicked()
                                    {
                                        relink.push((asset_path.clone(), candidate.clone()));
                                    }
                                });
                            }
                        }
                    });
                ui.label(
                    egui::RichText::new(
                        "They show as red placeholders and load on their own once the files \
                         appear, for example when syncing finishes. To use a different asset, \
                         select a placeholder and use Relink in its properties.",
                    )
                    .weak()
                    .small(),
                );
                ui.add_space(8.0);
            }

            if ui.button("OK").clicked() {
                dismiss = true;
            }
        });

    if dismiss {
        *report = MapAssetReport::default();
    }

    for (old_path, new_path) in relink {
        report.missing.retain(|(path, _)| *path != old_path);
        report.relinked.push((old_path.clone(), new_path.clone()));
        retarget_events.write(RetargetAssetRequest { old_path, new_path });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_path_names_machine_and_avoids_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let map = dir.path().join("cave.json");

        assert_eq!(
            copy_path(&map, Some("LAPTOP")),
            dir.path().join("cave (LAPTOP copy).json")
        );
        std::fs::write(dir.path().join("cave (LAPTOP copy).json"), "{}").unwrap();
        assert_eq!(
            copy_path(&map, Some("LAPTOP")),
            dir.path().join("cave (LAPTOP copy 2).json")
        );
        assert_eq!(copy_path(&map, None), dir.path().join("cave (copy).json"));
    }
}
