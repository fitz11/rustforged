use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::path::PathBuf;

use crate::assets::{AssetCategory, RefreshAssetLibrary};

#[derive(Resource, Default)]
pub struct AssetImportDialog {
    pub is_open: bool,
    pub selected_category: AssetCategory,
    pub files_to_import: Vec<PathBuf>,
    pub import_status: Option<String>,
}

pub fn asset_import_ui(
    mut contexts: EguiContexts,
    mut dialog: ResMut<AssetImportDialog>,
    mut refresh_events: MessageWriter<RefreshAssetLibrary>,
) -> Result {
    if !dialog.is_open {
        return Ok(());
    }

    let mut should_close = false;

    egui::Window::new("Import Assets")
        .collapsible(false)
        .resizable(true)
        .default_width(400.0)
        .show(contexts.ctx_mut()?, |ui| {
            ui.horizontal(|ui| {
                ui.label("Category:");
                egui::ComboBox::from_id_salt("import_category")
                    .selected_text(dialog.selected_category.display_name())
                    .show_ui(ui, |ui| {
                        for category in AssetCategory::all() {
                            let is_selected = dialog.selected_category == *category;
                            if ui
                                .selectable_label(is_selected, category.display_name())
                                .clicked()
                            {
                                dialog.selected_category = *category;
                            }
                        }
                    });
            });

            ui.separator();

            if ui.button("Browse Files...").clicked() {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif"])
                    .set_title("Select images to import")
                    .pick_files()
                {
                    dialog.files_to_import = paths;
                    dialog.import_status = None;
                }
            }

            if !dialog.files_to_import.is_empty() {
                ui.separator();
                ui.label(format!("Selected {} file(s):", dialog.files_to_import.len()));

                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for path in &dialog.files_to_import {
                            if let Some(name) = path.file_name() {
                                ui.label(name.to_string_lossy().to_string());
                            }
                        }
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Import").clicked() {
                        let dest_folder = PathBuf::from("assets/library")
                            .join(dialog.selected_category.folder_name());

                        // Ensure directory exists
                        if let Err(e) = std::fs::create_dir_all(&dest_folder) {
                            dialog.import_status = Some(format!("Failed to create directory: {}", e));
                            return;
                        }

                        let mut imported = 0;
                        let mut errors = Vec::new();

                        for src_path in &dialog.files_to_import {
                            if let Some(filename) = src_path.file_name() {
                                let dest_path = dest_folder.join(filename);

                                // Check for duplicates
                                if dest_path.exists() {
                                    errors.push(format!(
                                        "{}: file already exists",
                                        filename.to_string_lossy()
                                    ));
                                    continue;
                                }

                                match std::fs::copy(src_path, &dest_path) {
                                    Ok(_) => imported += 1,
                                    Err(e) => {
                                        errors.push(format!(
                                            "{}: {}",
                                            filename.to_string_lossy(),
                                            e
                                        ));
                                    }
                                }
                            }
                        }

                        if errors.is_empty() {
                            dialog.import_status =
                                Some(format!("Successfully imported {} file(s)", imported));
                            refresh_events.write(RefreshAssetLibrary);
                        } else {
                            dialog.import_status = Some(format!(
                                "Imported {} file(s). Errors:\n{}",
                                imported,
                                errors.join("\n")
                            ));
                            if imported > 0 {
                                refresh_events.write(RefreshAssetLibrary);
                            }
                        }

                        dialog.files_to_import.clear();
                    }

                    if ui.button("Clear").clicked() {
                        dialog.files_to_import.clear();
                        dialog.import_status = None;
                    }
                });
            }

            if let Some(status) = &dialog.import_status {
                ui.separator();
                ui.label(status);
            }

            ui.separator();

            if ui.button("Close").clicked() {
                should_close = true;
            }
        });

    if should_close {
        dialog.is_open = false;
        dialog.files_to_import.clear();
        dialog.import_status = None;
    }

    Ok(())
}
