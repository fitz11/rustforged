use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowCloseRequested};
use bevy_egui::{egui, EguiContexts};

use crate::assets::AssetLibrary;
use crate::config::{AppConfig, ConfigResetNotification, MissingMapWarning, SaveConfigRequest};
use crate::map::{
    AsyncMapOperation, LoadValidationWarning, MapLoadError, MapSaveError, NewMapRequest, OpenMaps,
    SaveMapRequest, SaveValidationWarning, UnsavedChangesDialog,
};

#[derive(Resource, Default)]
pub struct FileMenuState {
    pub show_new_confirmation: bool,
    pub show_save_name_dialog: bool,
    pub save_filename: String,
}

/// Renders the dialog windows for file operations (triggered from asset_browser menu)
pub fn file_menu_ui(
    mut contexts: EguiContexts,
    mut menu_state: ResMut<FileMenuState>,
    mut save_events: MessageWriter<SaveMapRequest>,
    mut new_events: MessageWriter<NewMapRequest>,
    load_error: Res<MapLoadError>,
    library: Res<AssetLibrary>,
) -> Result {
    // New map confirmation dialog
    if menu_state.show_new_confirmation {
        egui::Window::new("New Map")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                ui.label("Create a new map? Unsaved changes will be lost.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save First").clicked() {
                        // Open save dialog, keep new confirmation for after save
                        menu_state.show_save_name_dialog = true;
                        menu_state.show_new_confirmation = false;
                    }
                    if ui.button("Create New").clicked() {
                        new_events.write(NewMapRequest);
                        menu_state.show_new_confirmation = false;
                    }
                    if ui.button("Cancel").clicked() {
                        menu_state.show_new_confirmation = false;
                    }
                });
            });
    }

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
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        // Ensure maps directory exists
                        if !maps_dir.exists()
                            && let Err(e) = std::fs::create_dir_all(&maps_dir)
                        {
                            warn!("Failed to create maps directory: {}", e);
                        }

                        let filename = sanitize_filename(&menu_state.save_filename);
                        let path = maps_dir.join(format!("{}.json", filename));
                        save_events.write(SaveMapRequest { path });
                        menu_state.show_save_name_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        menu_state.show_save_name_dialog = false;
                    }
                });
            });
    }

    // Load error dialog
    if let Some(error) = &load_error.message {
        egui::Window::new("Load Error")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    ui.colored_label(egui::Color32::RED, error);
                });
            });
    }

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
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

/// Renders the unsaved changes confirmation dialog when closing the app
pub fn unsaved_changes_dialog_ui(
    mut contexts: EguiContexts,
    mut dialog: ResMut<UnsavedChangesDialog>,
    open_maps: Res<OpenMaps>,
    mut menu_state: ResMut<FileMenuState>,
    mut exit_events: MessageWriter<AppExit>,
) -> Result {
    if !dialog.show_close_confirmation {
        return Ok(());
    }

    egui::Window::new("Unsaved Changes")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.label("You have unsaved changes in:");
            ui.add_space(4.0);

            // List unsaved maps
            for map in open_maps.maps.values().filter(|m| m.is_dirty) {
                ui.label(egui::RichText::new(format!("  - {}", map.name)).strong());
            }

            ui.add_space(8.0);
            ui.label("Do you want to save before closing?");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Save All").clicked() {
                    // Trigger save dialog for each unsaved map
                    // For simplicity, just show the save dialog for the current map
                    menu_state.show_save_name_dialog = true;
                    dialog.show_close_confirmation = false;
                }

                if ui.button("Discard & Close").clicked() {
                    dialog.show_close_confirmation = false;
                    exit_events.write(AppExit::Success);
                }

                if ui.button("Cancel").clicked() {
                    dialog.show_close_confirmation = false;
                }
            });
        });

    Ok(())
}

/// System to intercept window close requests and show unsaved changes dialog if needed.
///
/// With `close_when_requested: false` on WindowPlugin, we must manually handle closing.
/// If there are unsaved changes, show a confirmation dialog. Otherwise, exit immediately.
pub fn handle_window_close(
    mut close_events: MessageReader<WindowCloseRequested>,
    mut dialog: ResMut<UnsavedChangesDialog>,
    open_maps: Res<OpenMaps>,
    mut exit_events: MessageWriter<AppExit>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    let primary_entity = primary_window.single().ok();

    for event in close_events.read() {
        // Only handle close requests for the primary window
        if Some(event.window) != primary_entity {
            continue;
        }

        // Check if any maps have unsaved changes
        if open_maps.maps.values().any(|m| m.is_dirty) {
            // Show confirmation dialog instead of closing
            dialog.show_close_confirmation = true;
        } else {
            // No unsaved changes, exit immediately
            exit_events.write(AppExit::Success);
        }
    }
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
                i.viewport()
                    .inner_rect
                    .unwrap_or(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(1920.0, 1080.0)))
            });
            ui.painter().rect_filled(
                rect,
                0.0,
                egui::Color32::from_black_alpha(100),
            );
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

            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
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

/// Renders the warning dialog about missing assets before save
pub fn save_validation_warning_ui(
    mut contexts: EguiContexts,
    mut warning: ResMut<SaveValidationWarning>,
    mut save_events: MessageWriter<SaveMapRequest>,
) -> Result {
    if !warning.show {
        return Ok(());
    }

    egui::Window::new("Missing Assets Warning")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.label("The following assets could not be found:");
            ui.add_space(8.0);

            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for path in &warning.missing_assets {
                    ui.label(egui::RichText::new(format!("  - {}", path)).weak());
                }
            });

            ui.add_space(8.0);
            ui.label("These items will appear as placeholders when the map is loaded.");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Save Anyway").clicked() {
                    if let Some(path) = warning.pending_save_path.take() {
                        save_events.write(SaveMapRequest { path });
                    }
                    warning.show = false;
                    warning.missing_assets.clear();
                }

                if ui.button("Cancel").clicked() {
                    warning.show = false;
                    warning.missing_assets.clear();
                    warning.pending_save_path = None;
                }
            });
        });

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

/// Renders the warning dialog when a map cannot be loaded due to missing assets
pub fn load_validation_warning_ui(
    mut contexts: EguiContexts,
    mut warning: ResMut<LoadValidationWarning>,
) -> Result {
    if !warning.show {
        return Ok(());
    }

    egui::Window::new("Missing Assets")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.label(egui::RichText::new("Cannot load map").strong());
            ui.add_space(4.0);

            if let Some(ref path) = warning.map_path {
                let map_name = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown");
                ui.label(
                    egui::RichText::new(format!("Map: {}", map_name))
                        .weak()
                        .small(),
                );
                ui.add_space(4.0);
            }

            ui.label("The following assets are not in the current library:");
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for asset_path in &warning.missing_assets {
                        ui.label(
                            egui::RichText::new(format!("  - {}", asset_path))
                                .color(egui::Color32::from_rgb(200, 100, 100)),
                        );
                    }
                });

            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(
                    "Import the missing assets to the library, or open a library that contains them.",
                )
                .weak()
                .small(),
            );
            ui.add_space(8.0);

            if ui.button("OK").clicked() {
                warning.show = false;
                warning.missing_assets.clear();
                warning.map_path = None;
            }
        });

    Ok(())
}
