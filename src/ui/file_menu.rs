use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::path::PathBuf;

use crate::map::{MapLoadError, NewMapRequest, SaveMapRequest};

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
