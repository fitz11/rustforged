use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::path::{Path, PathBuf};

use crate::assets::{AssetLibrary, RefreshAssetLibrary};
use crate::map::SavedMap;

#[derive(Resource, Default)]
pub struct AssetImportDialog {
    pub is_open: bool,
    pub files_to_import: Vec<PathBuf>,
    pub import_status: Option<String>,
    /// Files detected as invalid (neither image nor valid map)
    pub invalid_files: Vec<(PathBuf, String)>,
}

/// Check if a file is a valid image
fn is_image_file(path: &Path) -> bool {
    let extensions = ["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif"];
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Check if a file is a valid .json map file
fn validate_map_file(path: &Path) -> Result<(), String> {
    // Check extension
    if path.extension().and_then(|e| e.to_str()) != Some("json") {
        return Err("Not a .json file".to_string());
    }

    // Try to parse as SavedMap
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    serde_json::from_str::<SavedMap>(&content)
        .map_err(|e| format!("Invalid map format: {}", e))?;

    Ok(())
}

/// Categorize a file for import
#[derive(Debug)]
enum ImportFileType {
    Image,
    Map,
    Invalid(String),
}

fn categorize_import_file(path: &Path) -> ImportFileType {
    if is_image_file(path) {
        return ImportFileType::Image;
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str())
        && ext.to_lowercase() == "json"
    {
        return match validate_map_file(path) {
            Ok(()) => ImportFileType::Map,
            Err(e) => ImportFileType::Invalid(e),
        };
    }

    ImportFileType::Invalid("Unknown file type".to_string())
}

pub fn asset_import_ui(
    mut contexts: EguiContexts,
    mut dialog: ResMut<AssetImportDialog>,
    mut refresh_events: MessageWriter<RefreshAssetLibrary>,
    library: Res<AssetLibrary>,
) -> Result {
    if !dialog.is_open {
        return Ok(());
    }

    let mut should_close = false;

    egui::Window::new("Import Files")
        .collapsible(false)
        .resizable(true)
        .default_width(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.label(egui::RichText::new("Import files to library").size(14.0));
            ui.add_space(4.0);

            // Show destination info
            ui.label(
                egui::RichText::new(format!("Library: {}", library.metadata.name))
                    .weak()
                    .small(),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Images -> library root  |  Maps -> maps/")
                    .weak()
                    .small(),
            );
            ui.label(
                egui::RichText::new("Original filenames and extensions are preserved.")
                    .weak()
                    .small(),
            );

            ui.separator();

            // Browse button with updated filter
            if ui.button("Browse Files...").clicked()
                && let Some(paths) = rfd::FileDialog::new()
                    .add_filter(
                        "Supported Files",
                        &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif", "json"],
                    )
                    .add_filter(
                        "Images",
                        &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif"],
                    )
                    .add_filter("Map Files", &["json"])
                    .set_title("Select files to import")
                    .pick_files()
            {
                dialog.files_to_import = paths;
                dialog.import_status = None;
                dialog.invalid_files.clear();
            }

            if !dialog.files_to_import.is_empty() {
                ui.separator();

                // Categorize files (clone paths to avoid borrow issues)
                let mut images: Vec<PathBuf> = Vec::new();
                let mut maps: Vec<PathBuf> = Vec::new();
                let mut invalid_files: Vec<(PathBuf, String)> = Vec::new();

                for path in &dialog.files_to_import {
                    match categorize_import_file(path) {
                        ImportFileType::Image => images.push(path.clone()),
                        ImportFileType::Map => maps.push(path.clone()),
                        ImportFileType::Invalid(reason) => {
                            invalid_files.push((path.clone(), reason));
                        }
                    }
                }

                let file_count = dialog.files_to_import.len();

                ui.label(format!("Selected {} file(s):", file_count));

                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        if !images.is_empty() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Images ({}) -> library root:",
                                    images.len()
                                ))
                                .strong(),
                            );
                            for path in &images {
                                if let Some(name) = path.file_name() {
                                    ui.label(format!("  {}", name.to_string_lossy()));
                                }
                            }
                        }

                        if !maps.is_empty() {
                            if !images.is_empty() {
                                ui.add_space(4.0);
                            }
                            ui.label(
                                egui::RichText::new(format!("Maps ({}) -> maps/:", maps.len()))
                                    .strong(),
                            );
                            for path in &maps {
                                if let Some(name) = path.file_name() {
                                    ui.label(format!("  {}", name.to_string_lossy()));
                                }
                            }
                        }

                        if !invalid_files.is_empty() {
                            if !images.is_empty() || !maps.is_empty() {
                                ui.add_space(4.0);
                            }
                            ui.label(
                                egui::RichText::new(format!("Invalid ({}):", invalid_files.len()))
                                    .color(egui::Color32::RED),
                            );
                            for (path, reason) in &invalid_files {
                                if let Some(name) = path.file_name() {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "  {} - {}",
                                            name.to_string_lossy(),
                                            reason
                                        ))
                                        .color(egui::Color32::from_rgb(200, 100, 100)),
                                    );
                                }
                            }
                        }
                    });

                ui.separator();

                let can_import = !images.is_empty() || !maps.is_empty();

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(can_import, egui::Button::new("Import"))
                        .clicked()
                    {
                        let mut imported = 0;
                        let mut errors = Vec::new();

                        // Import images to library root
                        let image_dest = library.library_path.clone();
                        if !image_dest.exists()
                            && let Err(e) = std::fs::create_dir_all(&image_dest)
                        {
                            errors.push(format!("Failed to create library directory: {}", e));
                        }
                        if image_dest.exists() {
                            for src_path in &images {
                                if let Some(filename) = src_path.file_name() {
                                    let dest_path = image_dest.join(filename);
                                    if dest_path.exists() {
                                        errors.push(format!(
                                            "{}: file already exists",
                                            filename.to_string_lossy()
                                        ));
                                    } else {
                                        match std::fs::copy(src_path, &dest_path) {
                                            Ok(_) => imported += 1,
                                            Err(e) => errors.push(format!(
                                                "{}: {}",
                                                filename.to_string_lossy(),
                                                e
                                            )),
                                        }
                                    }
                                }
                            }
                        }

                        // Import maps to maps folder
                        let maps_dest = library.library_path.join("maps");
                        if let Err(e) = std::fs::create_dir_all(&maps_dest) {
                            errors.push(format!("Failed to create maps directory: {}", e));
                        } else {
                            for src_path in &maps {
                                if let Some(filename) = src_path.file_name() {
                                    let dest_path = maps_dest.join(filename);
                                    if dest_path.exists() {
                                        errors.push(format!(
                                            "{}: file already exists",
                                            filename.to_string_lossy()
                                        ));
                                    } else {
                                        match std::fs::copy(src_path, &dest_path) {
                                            Ok(_) => imported += 1,
                                            Err(e) => errors.push(format!(
                                                "{}: {}",
                                                filename.to_string_lossy(),
                                                e
                                            )),
                                        }
                                    }
                                }
                            }
                        }

                        // Set status
                        if errors.is_empty() {
                            dialog.import_status =
                                Some(format!("Successfully imported {} file(s)", imported));
                        } else {
                            dialog.import_status = Some(format!(
                                "Imported {} file(s). Errors:\n{}",
                                imported,
                                errors.join("\n")
                            ));
                        }

                        if imported > 0 {
                            refresh_events.write(RefreshAssetLibrary);
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
        dialog.invalid_files.clear();
        dialog.import_status = None;
    }

    Ok(())
}
