use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use bevy_egui::{egui, EguiContexts};
use std::path::PathBuf;

use crate::config::{AppConfig, MissingMapWarning, SaveConfigRequest};
use crate::map::{MapLoadError, NewMapRequest, OpenMaps, SaveMapRequest, UnsavedChangesDialog};

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
) -> Result {
    // New map confirmation dialog
    if menu_state.show_new_confirmation {
        egui::Window::new("New Map")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                ui.label("Create a new map? Unsaved changes will be lost.");
                ui.horizontal(|ui| {
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
        egui::Window::new("Save Map")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Map name:");
                    ui.text_edit_singleline(&mut menu_state.save_filename);
                });
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        let maps_dir = PathBuf::from("assets/maps");
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
                    .on_hover_text(path_str.as_ref());
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

/// System to intercept window close requests and show unsaved changes dialog if needed
pub fn handle_window_close(
    mut close_events: MessageReader<WindowCloseRequested>,
    mut dialog: ResMut<UnsavedChangesDialog>,
    open_maps: Res<OpenMaps>,
) {
    for _event in close_events.read() {
        // Check if any maps have unsaved changes
        if open_maps.maps.values().any(|m| m.is_dirty) {
            // Show confirmation dialog instead of closing
            dialog.show_close_confirmation = true;
            // Note: We can't prevent the window from closing in Bevy directly,
            // so we rely on the dialog to give the user a chance to save
        }
    }
}
