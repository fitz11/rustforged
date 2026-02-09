use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy_egui::{egui, EguiContexts};
use futures_lite::future;
use std::path::PathBuf;

use crate::assets::{AssetLibrary, UpdateLibraryMetadataRequest};
use crate::config::{AppConfig, SaveConfigRequest, SetDefaultLibraryRequest};

/// State for the settings dialog
#[derive(Resource, Default)]
pub struct SettingsDialogState {
    /// Whether the dialog is open
    pub is_open: bool,
    /// Edited default library path (as string for text editing)
    pub default_library_path: String,
    /// Whether changes have been made
    pub has_changes: bool,
    /// Edited library name
    pub library_name: String,
    /// Whether library name has been changed
    pub library_name_changed: bool,
    /// Pending async file dialog for browsing folders
    pub pending_browse: Option<Task<Option<PathBuf>>>,
}

impl SettingsDialogState {
    /// Initialize the dialog state from current config
    pub fn load_from_config(&mut self, config: &AppConfig) {
        self.default_library_path = config
            .data
            .default_library_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        self.has_changes = false;
        self.library_name_changed = false;
    }

    /// Load library info when dialog opens
    pub fn load_library_info(&mut self, library: &AssetLibrary) {
        self.library_name = library.metadata.name.clone();
        self.library_name_changed = false;
    }
}

/// Renders the settings dialog
pub fn settings_dialog_ui(
    mut contexts: EguiContexts,
    mut dialog_state: ResMut<SettingsDialogState>,
    mut config: ResMut<AppConfig>,
    library: Res<AssetLibrary>,
    mut set_default_events: MessageWriter<SetDefaultLibraryRequest>,
    mut save_events: MessageWriter<SaveConfigRequest>,
    mut metadata_events: MessageWriter<UpdateLibraryMetadataRequest>,
) -> Result {
    // Poll pending browse task (before early return so cleanup happens even if closed)
    if let Some(ref mut task) = dialog_state.pending_browse
        && let Some(result) = future::block_on(future::poll_once(task))
    {
        dialog_state.pending_browse = None;
        if let Some(path) = result {
            dialog_state.default_library_path = path.to_string_lossy().to_string();
            dialog_state.has_changes = true;
        }
    }

    if !dialog_state.is_open {
        return Ok(());
    }

    let mut should_close = false;
    let mut should_save = false;
    let mut should_browse = false;
    let mut should_clear = false;

    egui::Window::new("Settings")
        .collapsible(false)
        .resizable(true)
        .min_width(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.heading("Application Settings");
            ui.add_space(12.0);

            // Current Library section
            ui.group(|ui| {
                ui.label(egui::RichText::new("Current Library").strong());
                ui.add_space(8.0);

                // Show library path
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.label(
                        egui::RichText::new(&*library.library_path.to_string_lossy()).weak(),
                    );
                });

                ui.add_space(4.0);

                // Editable library name
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    let response =
                        ui.add(egui::TextEdit::singleline(&mut dialog_state.library_name));
                    if response.changed() {
                        dialog_state.library_name_changed = true;
                        dialog_state.has_changes = true;
                    }
                });
            });

            ui.add_space(12.0);

            // Default Library Path section
            ui.group(|ui| {
                ui.label(egui::RichText::new("Default Library").strong());
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Path:");
                });

                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut dialog_state.default_library_path)
                            .desired_width(280.0)
                            .hint_text("No default library set"),
                    );
                    if response.changed() {
                        dialog_state.has_changes = true;
                    }

                    if ui.button("Browse...").clicked() {
                        should_browse = true;
                    }

                    if ui.button("Clear").clicked() {
                        should_clear = true;
                    }
                });

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "The default library will be opened automatically when the application starts.",
                    )
                    .weak()
                    .small(),
                );
            });

            ui.add_space(12.0);

            // Recent Libraries section (read-only display)
            ui.group(|ui| {
                ui.label(egui::RichText::new("Recent Libraries").strong());
                ui.add_space(8.0);

                if config.data.recent_libraries.is_empty() {
                    ui.label(egui::RichText::new("No recent libraries").weak().italics());
                } else {
                    for (i, path) in config.data.recent_libraries.iter().enumerate() {
                        let display = path.to_string_lossy();
                        ui.label(format!("{}. {}", i + 1, display));
                    }
                }
            });

            ui.add_space(12.0);

            // Last Map section (read-only display)
            ui.group(|ui| {
                ui.label(egui::RichText::new("Last Opened Map").strong());
                ui.add_space(8.0);

                if let Some(ref path) = config.data.last_map_path {
                    let display = path.to_string_lossy();
                    ui.label(&*display);
                } else {
                    ui.label(egui::RichText::new("No map recorded").weak().italics());
                }
            });

            ui.add_space(16.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(dialog_state.has_changes, egui::Button::new("Save"))
                    .clicked()
                {
                    should_save = true;
                }

                if ui.button("Cancel").clicked() {
                    should_close = true;
                }
            });
        });

    // Handle browse button - spawn async dialog
    if should_browse && dialog_state.pending_browse.is_none() {
        let task_pool = AsyncComputeTaskPool::get();
        dialog_state.pending_browse = Some(task_pool.spawn(async {
            rfd::AsyncFileDialog::new()
                .set_title("Select Default Asset Library")
                .pick_folder()
                .await
                .map(|h| h.path().to_path_buf())
        }));
    }

    // Handle clear button
    if should_clear {
        dialog_state.default_library_path.clear();
        dialog_state.has_changes = true;
    }

    // Handle save
    if should_save {
        // Save library name if changed
        if dialog_state.library_name_changed {
            metadata_events.write(UpdateLibraryMetadataRequest {
                name: dialog_state.library_name.clone(),
            });
            dialog_state.library_name_changed = false;
        }

        let new_path = if dialog_state.default_library_path.is_empty() {
            None
        } else {
            Some(PathBuf::from(&dialog_state.default_library_path))
        };

        // Update config directly
        config.data.default_library_path = new_path.clone();
        config.dirty = true;
        save_events.write(SaveConfigRequest);

        // If a path was set, also trigger the library request for immediate effect
        if let Some(path) = new_path {
            set_default_events.write(SetDefaultLibraryRequest { path });
        }

        dialog_state.has_changes = false;
        should_close = true;
    }

    // Handle close
    if should_close {
        dialog_state.is_open = false;
        dialog_state.pending_browse = None;
        // Reset state from config
        dialog_state.load_from_config(&config);
        dialog_state.load_library_info(&library);
    }

    Ok(())
}
