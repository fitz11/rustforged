use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use bevy_egui::{EguiContexts, egui};
use std::path::{Path, PathBuf};

use crate::assets::{AssetLibrary, RefreshAssetLibrary};
use crate::common::poll_task;
use crate::map::SavedMap;

#[derive(Resource, Default)]
pub struct AssetImportDialog {
    pub is_open: bool,
    /// The picked files, sorted into images, maps and files that can't be imported
    pub selection: Option<ImportSelection>,
    pub import_status: Option<String>,
    /// File picker, then sorting the picked files (reading each map to check it), off the
    /// UI thread
    pub pending_browse: Option<Task<Option<ImportSelection>>>,
    /// Copying the files into the library, off the UI thread
    pub pending_import: Option<Task<ImportOutcome>>,
}

/// Picked files, sorted once when they're picked rather than every frame.
#[derive(Default)]
pub struct ImportSelection {
    pub images: Vec<PathBuf>,
    pub maps: Vec<PathBuf>,
    /// Files that are neither an image nor a valid map, with why
    pub invalid: Vec<(PathBuf, String)>,
}

impl ImportSelection {
    fn sort(paths: Vec<PathBuf>) -> Self {
        let mut selection = Self::default();
        for path in paths {
            match categorize_import_file(&path) {
                ImportFileType::Image => selection.images.push(path),
                ImportFileType::Map => selection.maps.push(path),
                ImportFileType::Invalid(reason) => selection.invalid.push((path, reason)),
            }
        }
        selection
    }

    fn total(&self) -> usize {
        self.images.len() + self.maps.len() + self.invalid.len()
    }
}

/// What an import copied, and what it couldn't
pub struct ImportOutcome {
    imported: usize,
    errors: Vec<String>,
}

/// Copies `sources` into `dest_dir`, never replacing a file that's already there.
fn copy_into(sources: &[PathBuf], dest_dir: &Path, outcome: &mut ImportOutcome) {
    if let Err(e) = std::fs::create_dir_all(dest_dir) {
        outcome
            .errors
            .push(format!("Failed to create {}: {}", dest_dir.display(), e));
        return;
    }
    for source in sources {
        let Some(filename) = source.file_name() else {
            continue;
        };
        let name = filename.to_string_lossy();
        let result = std::fs::File::open(source).and_then(|mut from| {
            let mut to = std::fs::File::create_new(dest_dir.join(filename))?;
            std::io::copy(&mut from, &mut to)
        });
        match result {
            Ok(_) => outcome.imported += 1,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                outcome
                    .errors
                    .push(format!("{}: file already exists", name));
            }
            Err(e) => outcome.errors.push(format!("{}: {}", name, e)),
        }
    }
}

/// Images go to the library root and maps to its maps folder.
fn import_files(images: Vec<PathBuf>, maps: Vec<PathBuf>, library_path: PathBuf) -> ImportOutcome {
    let mut outcome = ImportOutcome {
        imported: 0,
        errors: Vec::new(),
    };
    copy_into(&images, &library_path, &mut outcome);
    copy_into(&maps, &library_path.join("maps"), &mut outcome);
    outcome
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

    serde_json::from_str::<SavedMap>(&content).map_err(|e| format!("Invalid map format: {}", e))?;

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
    // Poll pending browse task (before early return so cleanup happens even if closed)
    if let Some(ref mut task) = dialog.pending_browse
        && let Some(result) = poll_task(task)
    {
        dialog.pending_browse = None;
        if let Some(selection) = result {
            dialog.selection = Some(selection);
            dialog.import_status = None;
        }
    }

    // Poll the copy
    if let Some(ref mut task) = dialog.pending_import
        && let Some(outcome) = poll_task(task)
    {
        dialog.pending_import = None;
        dialog.import_status = Some(if outcome.errors.is_empty() {
            format!("Successfully imported {} file(s)", outcome.imported)
        } else {
            format!(
                "Imported {} file(s). Errors:\n{}",
                outcome.imported,
                outcome.errors.join("\n")
            )
        });
        if outcome.imported > 0 {
            refresh_events.write(RefreshAssetLibrary);
        }
    }

    if !dialog.is_open {
        return Ok(());
    }

    let mut should_close = false;
    let mut start_import = false;
    let mut clear = false;

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

            let busy = dialog.pending_browse.is_some() || dialog.pending_import.is_some();
            if ui
                .add_enabled(!busy, egui::Button::new("Browse Files..."))
                .clicked()
            {
                dialog.pending_browse = Some(IoTaskPool::get().spawn(async {
                    let picked = rfd::AsyncFileDialog::new()
                        .add_filter(
                            "Supported Files",
                            &[
                                "png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif", "json",
                            ],
                        )
                        .add_filter(
                            "Images",
                            &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif"],
                        )
                        .add_filter("Map Files", &["json"])
                        .set_title("Select files to import")
                        .pick_files()
                        .await?;
                    let paths = picked.iter().map(|h| h.path().to_path_buf()).collect();
                    Some(ImportSelection::sort(paths))
                }));
            }

            if dialog.pending_import.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Importing...");
                });
            }

            if let Some(selection) = &dialog.selection {
                ui.separator();
                ui.label(format!("Selected {} file(s):", selection.total()));

                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| render_selection(ui, selection));

                ui.separator();

                let can_import =
                    !busy && (!selection.images.is_empty() || !selection.maps.is_empty());

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(can_import, egui::Button::new("Import"))
                        .clicked()
                    {
                        start_import = true;
                    }

                    if ui.button("Clear").clicked() {
                        clear = true;
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

    if start_import && let Some(selection) = dialog.selection.take() {
        let library_path = library.library_path.clone();
        dialog.pending_import =
            Some(IoTaskPool::get().spawn(async move {
                import_files(selection.images, selection.maps, library_path)
            }));
    }
    if clear {
        dialog.selection = None;
        dialog.import_status = None;
    }

    if should_close {
        dialog.is_open = false;
        dialog.selection = None;
        dialog.import_status = None;
        dialog.pending_browse = None;
    }

    Ok(())
}

/// Lists the picked files by what happens to them.
fn render_selection(ui: &mut egui::Ui, selection: &ImportSelection) {
    let file_name = |path: &PathBuf| {
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };

    if !selection.images.is_empty() {
        ui.label(
            egui::RichText::new(format!(
                "Images ({}) -> library root:",
                selection.images.len()
            ))
            .strong(),
        );
        for path in &selection.images {
            ui.label(format!("  {}", file_name(path)));
        }
    }

    if !selection.maps.is_empty() {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!("Maps ({}) -> maps/:", selection.maps.len())).strong(),
        );
        for path in &selection.maps {
            ui.label(format!("  {}", file_name(path)));
        }
    }

    if !selection.invalid.is_empty() {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!("Invalid ({}):", selection.invalid.len()))
                .color(egui::Color32::RED),
        );
        for (path, reason) in &selection.invalid {
            ui.label(
                egui::RichText::new(format!("  {} - {}", file_name(path), reason))
                    .color(egui::Color32::from_rgb(200, 100, 100)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_never_replaces_library_files() {
        let source = tempfile::tempdir().unwrap();
        let library = tempfile::tempdir().unwrap();
        let orc = source.path().join("orc.png");
        std::fs::write(&orc, "new orc").unwrap();
        std::fs::write(library.path().join("orc.png"), "my orc").unwrap();
        let goblin = source.path().join("goblin.png");
        std::fs::write(&goblin, "goblin").unwrap();

        let outcome = import_files(vec![orc, goblin], vec![], library.path().to_path_buf());

        assert_eq!(outcome.imported, 1);
        assert_eq!(outcome.errors.len(), 1);
        assert_eq!(
            std::fs::read_to_string(library.path().join("orc.png")).unwrap(),
            "my orc"
        );
    }

    #[test]
    fn test_selection_sorts_files_once() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("a.png");
        let bad_map = dir.path().join("b.json");
        std::fs::write(&image, "x").unwrap();
        std::fs::write(&bad_map, "not a map").unwrap();

        let selection = ImportSelection::sort(vec![image, bad_map, dir.path().join("c.txt")]);

        assert_eq!(selection.images.len(), 1);
        assert!(selection.maps.is_empty());
        assert_eq!(selection.invalid.len(), 2);
    }
}
