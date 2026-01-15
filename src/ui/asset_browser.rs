use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle, EguiUserTextures};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use std::collections::HashSet;

use crate::assets::{
    create_and_open_library, get_image_dimensions, load_thumbnail, open_library_directory,
    AssetLibrary, LibraryAsset, RenameAssetRequest, SelectedAsset, ThumbnailCache,
    UpdateLibraryMetadataRequest, THUMBNAIL_SIZE,
};
use crate::constants::MAX_THUMBNAILS_PER_FRAME;
use crate::config::{AppConfig, SetDefaultLibraryRequest};
use crate::editor::{CurrentTool, EditorTool};
use crate::map::{CurrentMapFile, LoadMapRequest, MapData, MapDirtyState, OpenMaps, SwitchMapRequest};

use super::asset_import::AssetImportDialog;
use super::file_menu::FileMenuState;
use super::settings_dialog::SettingsDialogState;

const LIBRARY_METADATA_FILE: &str = ".library.json";

/// Bundle of map-related resources and event writers
#[derive(SystemParam)]
pub struct MapResources<'w> {
    pub map_data: ResMut<'w, MapData>,
    pub current_map_file: Res<'w, CurrentMapFile>,
    pub dirty_state: ResMut<'w, MapDirtyState>,
    pub open_maps: Res<'w, OpenMaps>,
    pub load_events: MessageWriter<'w, LoadMapRequest>,
    pub switch_events: MessageWriter<'w, SwitchMapRequest>,
}

/// Bundle of dialog state resources
#[derive(SystemParam)]
pub struct DialogStates<'w> {
    pub menu_state: ResMut<'w, FileMenuState>,
    pub import_dialog: ResMut<'w, AssetImportDialog>,
    pub settings_state: ResMut<'w, SettingsDialogState>,
}

/// Scan the maps directory and return sorted list of map names (without extension)
fn scan_maps_directory(maps_dir: &std::path::Path) -> Vec<(String, PathBuf)> {
    if !maps_dir.exists() {
        return Vec::new();
    }

    let mut maps: Vec<(String, PathBuf)> = std::fs::read_dir(maps_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension().and_then(|ext| ext.to_str()) == Some("json")
                })
                .map(|e| {
                    let path = e.path();
                    let name = path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    (name, path)
                })
                .collect()
        })
        .unwrap_or_default();

    maps.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    maps
}

/// Export an entire library directory to a zip file
fn export_library_to_zip(library_path: &Path, dest_path: &Path) -> Result<(), String> {
    let file = File::create(dest_path).map_err(|e| format!("Failed to create zip file: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Walk the library directory recursively
    fn add_directory_to_zip(
        zip: &mut ZipWriter<File>,
        base_path: &Path,
        current_path: &Path,
        options: SimpleFileOptions,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(current_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(base_path)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;
            let relative_str = relative_path.to_string_lossy();

            if path.is_dir() {
                // Add directory entry
                zip.add_directory(format!("{}/", relative_str), options)
                    .map_err(|e| format!("Failed to add directory to zip: {}", e))?;
                // Recurse into subdirectory
                add_directory_to_zip(zip, base_path, &path, options)?;
            } else {
                // Add file
                zip.start_file(relative_str.to_string(), options)
                    .map_err(|e| format!("Failed to start file in zip: {}", e))?;
                let mut file =
                    File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)
                    .map_err(|e| format!("Failed to read file: {}", e))?;
                zip.write_all(&buffer)
                    .map_err(|e| format!("Failed to write file to zip: {}", e))?;
            }
        }
        Ok(())
    }

    add_directory_to_zip(&mut zip, library_path, library_path, options)?;
    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;
    Ok(())
}

/// Import a library from a zip file to a destination directory
fn import_library_from_zip(zip_path: &Path, dest_path: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Failed to open zip file: {}", e))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    // First pass: check if .library.json exists
    let mut has_library_metadata = false;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;
        let name = entry.name();
        // Check if .library.json is at the root level
        if name == LIBRARY_METADATA_FILE || name == format!("{}/", LIBRARY_METADATA_FILE) {
            has_library_metadata = true;
            break;
        }
    }

    if !has_library_metadata {
        return Err(
            "Invalid library archive: missing .library.json file.\n\n\
             This zip file does not contain a valid asset library."
                .to_string(),
        );
    }

    // Create destination directory
    std::fs::create_dir_all(dest_path)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;

    // Extract all files
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;
        let outpath = dest_path.join(entry.name());

        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            // Ensure parent directory exists
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
        }
    }

    Ok(())
}

/// System that loads thumbnails and registers them with egui.
/// Runs in Update before the egui pass to avoid timing issues.
pub fn load_and_register_thumbnails(
    library: Res<AssetLibrary>,
    mut thumbnail_cache: ResMut<ThumbnailCache>,
    mut images: ResMut<Assets<Image>>,
    mut egui_textures: ResMut<EguiUserTextures>,
) {
    // Load a limited number of new thumbnails per frame to avoid stuttering
    let assets_to_load: Vec<PathBuf> = library
        .assets
        .iter()
        .filter(|a| {
            !thumbnail_cache.thumbnails.contains_key(&a.full_path)
                && !thumbnail_cache.has_failed(&a.full_path)
        })
        .take(MAX_THUMBNAILS_PER_FRAME)
        .map(|a| a.full_path.clone())
        .collect();

    for path in assets_to_load {
        if let Some(thumb_image) = load_thumbnail(&path) {
            let handle = images.add(thumb_image);
            thumbnail_cache.thumbnails.insert(path, handle);
        } else {
            thumbnail_cache.failed.insert(path, ());
        }
    }

    // Register any thumbnails that don't have texture IDs yet
    let to_register: Vec<PathBuf> = thumbnail_cache
        .thumbnails
        .keys()
        .filter(|path| !thumbnail_cache.texture_ids.contains_key(*path))
        .cloned()
        .collect();

    for path in to_register {
        if let Some(handle) = thumbnail_cache.thumbnails.get(&path) {
            let texture_id = egui_textures.add_image(EguiTextureHandle::Weak(handle.id()));
            thumbnail_cache.texture_ids.insert(path, texture_id);
        }
    }
}

#[derive(Resource)]
pub struct AssetBrowserState {
    /// Currently selected folder path (empty string = root)
    pub selected_folder: String,
    /// Cached list of discovered folders in the library
    pub discovered_folders: Vec<String>,
    /// Whether the library info section is expanded
    pub library_expanded: bool,
    /// Cached dimensions for the selected asset
    pub selected_dimensions: Option<(u32, u32)>,
    /// Path of the asset for which dimensions are cached
    pub cached_dimensions_path: Option<PathBuf>,
    /// Last known library path (to detect changes and clear thumbnail cache)
    pub last_library_path: Option<PathBuf>,
    /// Whether the "set as default" dialog is shown
    pub show_set_default_dialog: bool,
    /// Path for the "set as default" dialog
    pub set_default_dialog_path: Option<PathBuf>,
    /// Checkbox state for "set as default" dialog
    pub set_as_default_checked: bool,
    /// Cached list of available maps in the library
    pub cached_maps: Vec<(String, PathBuf)>,
    /// Last path used for map scanning (to detect changes)
    pub last_maps_scan_path: Option<PathBuf>,
    /// Error message for library import failures
    pub library_import_error: Option<String>,
    /// Success message for export/import operations
    pub library_operation_success: Option<String>,
    /// Whether the rename dialog is open
    pub rename_dialog_open: bool,
    /// New name input for rename dialog
    pub rename_new_name: String,
    /// Error message for rename operation
    pub rename_error: Option<String>,
    /// Whether the rename map dialog is open
    pub rename_map_dialog_open: bool,
    /// New name input for rename map dialog
    pub rename_map_new_name: String,
    /// Whether the rename library dialog is open
    pub rename_library_dialog_open: bool,
    /// New name input for rename library dialog
    pub rename_library_new_name: String,
    /// Whether the move asset dialog is open
    pub move_dialog_open: bool,
    /// Error message for move operation
    pub move_error: Option<String>,
    /// New folder name input for move dialog
    pub move_new_folder_name: String,
}

impl Default for AssetBrowserState {
    fn default() -> Self {
        Self {
            selected_folder: String::new(),
            discovered_folders: Vec::new(),
            library_expanded: true,
            selected_dimensions: None,
            cached_dimensions_path: None,
            last_library_path: None,
            show_set_default_dialog: false,
            set_default_dialog_path: None,
            set_as_default_checked: false,
            cached_maps: Vec::new(),
            last_maps_scan_path: None,
            library_import_error: None,
            library_operation_success: None,
            rename_dialog_open: false,
            rename_new_name: String::new(),
            rename_error: None,
            rename_map_dialog_open: false,
            rename_map_new_name: String::new(),
            rename_library_dialog_open: false,
            rename_library_new_name: String::new(),
            move_dialog_open: false,
            move_error: None,
            move_new_folder_name: String::new(),
        }
    }
}

/// Discover all folders in the library from asset paths
fn discover_folders(library: &AssetLibrary) -> Vec<String> {
    let mut folders: HashSet<String> = HashSet::new();
    for asset in &library.assets {
        if !asset.folder_path.is_empty() {
            // Add the folder and all parent folders
            let mut path = String::new();
            for component in asset.folder_path.split(['/', '\\']) {
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(component);
                folders.insert(path.clone());
            }
        }
    }
    let mut sorted: Vec<String> = folders.into_iter().collect();
    sorted.sort();
    sorted
}

/// Rename an asset file and update all map files that reference it
fn rename_asset(
    old_path: &Path,
    new_name: &str,
    library_path: &Path,
) -> Result<(PathBuf, String, String), String> {
    // Validate new name
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    // Check for invalid characters
    if new_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("Name contains invalid characters".to_string());
    }

    let extension = old_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let parent = old_path.parent().ok_or("Invalid file path")?;
    let new_filename = if extension.is_empty() {
        new_name.to_string()
    } else {
        format!("{}.{}", new_name, extension)
    };

    let new_path = parent.join(&new_filename);

    // Check if target already exists
    if new_path.exists() && new_path != old_path {
        return Err("A file with that name already exists".to_string());
    }

    // Rename the file
    std::fs::rename(old_path, &new_path)
        .map_err(|e| format!("Failed to rename file: {}", e))?;

    // Calculate old and new relative paths for map updates
    let old_relative = if library_path.starts_with("assets/library") {
        // Internal library - use relative path
        let category_folder = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let old_filename = old_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        format!("library/{}/{}", category_folder, old_filename)
    } else {
        old_path.to_string_lossy().to_string()
    };

    let new_relative = if library_path.starts_with("assets/library") {
        let category_folder = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
        format!("library/{}/{}", category_folder, new_filename)
    } else {
        new_path.to_string_lossy().to_string()
    };

    // Update all map files in the library's maps folder
    let maps_dir = library_path.join("maps");
    if maps_dir.exists() {
        update_asset_paths_in_maps(&maps_dir, &old_relative, &new_relative)?;
    }

    Ok((new_path, old_relative, new_relative))
}

/// Update asset_path references in all map files
fn update_asset_paths_in_maps(maps_dir: &Path, old_path: &str, new_path: &str) -> Result<(), String> {
    let entries = std::fs::read_dir(maps_dir)
        .map_err(|e| format!("Failed to read maps directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            // Read the map file
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read map file {:?}: {}", path, e))?;

            // Check if this map references the old asset path
            if content.contains(old_path) {
                // Replace the old path with the new path
                let updated = content.replace(old_path, new_path);

                // Write back
                std::fs::write(&path, updated)
                    .map_err(|e| format!("Failed to update map file {:?}: {}", path, e))?;

                info!("Updated asset path in map: {:?}", path);
            }
        }
    }

    Ok(())
}

/// Move an asset file to a different folder and update all map files that reference it
fn move_asset(
    old_path: &Path,
    target_folder: &str,
    library_path: &Path,
) -> Result<(PathBuf, String, String), String> {
    // Get the filename
    let filename = old_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file path")?;

    // Build the new path in the target folder
    let new_folder = if target_folder.is_empty() {
        library_path.to_path_buf()
    } else {
        library_path.join(target_folder)
    };
    let new_path = new_folder.join(filename);

    // Check if target already exists
    if new_path.exists() {
        let target_display = if target_folder.is_empty() {
            "library root".to_string()
        } else {
            target_folder.to_string()
        };
        return Err(format!(
            "A file named '{}' already exists in {}",
            filename, target_display
        ));
    }

    // Ensure target folder exists
    if !new_folder.exists() {
        std::fs::create_dir_all(&new_folder)
            .map_err(|e| format!("Failed to create target folder: {}", e))?;
    }

    // Calculate relative paths for map updates
    let old_relative = old_path
        .strip_prefix(library_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| old_path.to_string_lossy().to_string());

    let new_relative = new_path
        .strip_prefix(library_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| new_path.to_string_lossy().to_string());

    // Move the file
    std::fs::rename(old_path, &new_path)
        .map_err(|e| format!("Failed to move file: {}", e))?;

    // Update all map files in the library's maps folder
    let maps_dir = library_path.join("maps");
    if maps_dir.exists() {
        update_asset_paths_in_maps(&maps_dir, &old_relative, &new_relative)?;
    }

    Ok((new_path, old_relative, new_relative))
}

#[allow(clippy::too_many_arguments)]
pub fn asset_browser_ui(
    mut contexts: EguiContexts,
    mut library: ResMut<AssetLibrary>,
    mut selected_asset: ResMut<SelectedAsset>,
    mut browser_state: ResMut<AssetBrowserState>,
    mut current_tool: ResMut<CurrentTool>,
    mut thumbnail_cache: ResMut<ThumbnailCache>,
    config: Res<AppConfig>,
    mut set_default_events: MessageWriter<SetDefaultLibraryRequest>,
    mut rename_events: MessageWriter<RenameAssetRequest>,
    mut library_metadata_events: MessageWriter<UpdateLibraryMetadataRequest>,
    mut map_res: MapResources,
    mut dialogs: DialogStates,
) -> Result {
    // Clear thumbnail cache if library path changed
    let current_path = library.library_path.clone();
    if browser_state.last_library_path.as_ref() != Some(&current_path) {
        thumbnail_cache.clear();
        browser_state.last_library_path = Some(current_path);
    }

    // Handle F2 key to open rename dialog for selected asset
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.input(|i| i.key_pressed(egui::Key::F2))
            && !browser_state.rename_dialog_open
            && let Some(ref asset) = selected_asset.asset
        {
            browser_state.rename_new_name = asset.name.clone();
            browser_state.rename_error = None;
            browser_state.rename_dialog_open = true;
        }

        // Handle F3 key to open rename map dialog
        if ctx.input(|i| i.key_pressed(egui::Key::F3))
            && !browser_state.rename_map_dialog_open
        {
            browser_state.rename_map_new_name = map_res.map_data.name.clone();
            browser_state.rename_map_dialog_open = true;
        }

        // Handle F4 key to open rename library dialog
        if ctx.input(|i| i.key_pressed(egui::Key::F4))
            && !browser_state.rename_library_dialog_open
        {
            browser_state.rename_library_new_name = library.metadata.name.clone();
            browser_state.rename_library_dialog_open = true;
        }

        // Handle Escape to close rename dialogs
        if browser_state.rename_dialog_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            browser_state.rename_dialog_open = false;
            browser_state.rename_error = None;
        }
        if browser_state.rename_map_dialog_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            browser_state.rename_map_dialog_open = false;
        }
        if browser_state.rename_library_dialog_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            browser_state.rename_library_dialog_open = false;
        }
    }

    egui::SidePanel::left("asset_browser")
        .default_width(220.0)
        .show(contexts.ctx_mut()?, |ui| {
            // =========================================
            // ASSET LIBRARY SECTION
            // =========================================
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let toggle_text = if browser_state.library_expanded {
                    "v"
                } else {
                    ">"
                };
                if ui.button(toggle_text).clicked() {
                    browser_state.library_expanded = !browser_state.library_expanded;
                }
                // Show library name from metadata
                ui.label(egui::RichText::new(&library.metadata.name).heading().size(18.0));
            });
            ui.add_space(2.0);

            // Show current library path (truncated if too long)
            let path_str = library.library_path.to_string_lossy();
            let display_path = if path_str.len() > 30 {
                format!("...{}", &path_str[path_str.len() - 27..])
            } else {
                path_str.to_string()
            };
            ui.label(egui::RichText::new(display_path).small().weak())
                .on_hover_text(path_str.as_ref());

            // Show error if any
            if let Some(ref error) = library.error {
                ui.colored_label(egui::Color32::RED, egui::RichText::new(error).small());
            }

            // Library management and subsections (shown when expanded)
            if browser_state.library_expanded {
                ui.add_space(6.0);

                // Library management buttons
                ui.horizontal(|ui| {
                    if ui.add_sized([65.0, 24.0], egui::Button::new("Open...")).clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .set_title("Open Asset Library")
                            .pick_folder()
                    {
                        // Try to open the library
                        if let Err(e) = open_library_directory(&mut library, path.clone()) {
                            warn!("Failed to open library: {}", e);
                        } else {
                            // Show the "set as default" dialog
                            browser_state.show_set_default_dialog = true;
                            browser_state.set_default_dialog_path = Some(path);
                            browser_state.set_as_default_checked = false;
                        }
                    }

                    if ui.add_sized([65.0, 24.0], egui::Button::new("New...")).clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .set_title("Create New Asset Library")
                            .pick_folder()
                    {
                        if let Err(e) = create_and_open_library(&mut library, path.clone()) {
                            warn!("Failed to create library: {}", e);
                        } else {
                            // Show the "set as default" dialog
                            browser_state.show_set_default_dialog = true;
                            browser_state.set_default_dialog_path = Some(path);
                            browser_state.set_as_default_checked = false;
                        }
                    }

                    if ui.add_sized([65.0, 24.0], egui::Button::new("Rename"))
                        .on_hover_text("Rename library (F4)")
                        .clicked()
                    {
                        browser_state.rename_library_new_name = library.metadata.name.clone();
                        browser_state.rename_library_dialog_open = true;
                    }
                });

                // Export/Import library buttons
                ui.horizontal(|ui| {
                    // Export library to zip
                    if ui.add_sized([70.0, 24.0], egui::Button::new("Export...")).clicked()
                        && let Some(dest_path) = rfd::FileDialog::new()
                            .set_title("Export Library as Zip")
                            .set_file_name(format!("{}.zip", library.metadata.name))
                            .add_filter("Zip Archive", &["zip"])
                            .save_file()
                    {
                        match export_library_to_zip(&library.library_path, &dest_path) {
                            Ok(()) => {
                                browser_state.library_operation_success =
                                    Some(format!("Library exported to:\n{}", dest_path.display()));
                            }
                            Err(e) => {
                                browser_state.library_import_error = Some(e);
                            }
                        }
                    }

                    // Import library from zip
                    if ui.add_sized([70.0, 24.0], egui::Button::new("Import...")).clicked()
                        && let Some(zip_path) = rfd::FileDialog::new()
                            .set_title("Import Library from Zip")
                            .add_filter("Zip Archive", &["zip"])
                            .pick_file()
                        && let Some(dest_path) = rfd::FileDialog::new()
                            .set_title("Select Destination Folder")
                            .pick_folder()
                    {
                        match import_library_from_zip(&zip_path, &dest_path) {
                            Ok(()) => {
                                // Open the imported library
                                if let Err(e) = open_library_directory(&mut library, dest_path.clone()) {
                                    browser_state.library_import_error =
                                        Some(format!("Library extracted but failed to open: {}", e));
                                } else {
                                    browser_state.library_operation_success =
                                        Some("Library imported successfully!".to_string());
                                    browser_state.show_set_default_dialog = true;
                                    browser_state.set_default_dialog_path = Some(dest_path);
                                    browser_state.set_as_default_checked = false;
                                }
                            }
                            Err(e) => {
                                browser_state.library_import_error = Some(e);
                            }
                        }
                    }
                });

                ui.add_space(10.0);

                // Maps subsection
                ui.label(egui::RichText::new("Maps").size(13.0).strong());
                ui.separator();

                // Show open maps with unsaved indicators
                let mut map_to_switch: Option<u64> = None;

                if !map_res.open_maps.maps.is_empty() {
                    ui.label(egui::RichText::new("Open:").size(12.0).weak());

                    // Sort maps by ID to maintain consistent order
                    let mut sorted_maps: Vec<_> = map_res.open_maps.maps.values().collect();
                    sorted_maps.sort_by_key(|m| m.id);

                    for map in sorted_maps {
                        let is_active = map_res.open_maps.active_map_id == Some(map.id);
                        let display_name = if map.is_dirty {
                            format!("{}*", map.name)
                        } else {
                            map.name.clone()
                        };

                        ui.horizontal(|ui| {
                            if is_active {
                                ui.label(egui::RichText::new(">").size(10.0));
                            } else {
                                ui.add_space(12.0);
                            }

                            let text = if is_active {
                                egui::RichText::new(&display_name).size(12.0).strong()
                            } else {
                                egui::RichText::new(&display_name).size(12.0)
                            };

                            // Make non-active maps clickable
                            if is_active {
                                ui.label(text);
                            } else if ui
                                .add(egui::Button::new(text).frame(false))
                                .on_hover_text("Click to switch to this map")
                                .clicked()
                            {
                                map_to_switch = Some(map.id);
                            }

                            // Show path hint if saved
                            if map.path.is_some() && map.is_dirty {
                                ui.label(egui::RichText::new("(modified)").size(10.0).weak().italics());
                            } else if map.path.is_none() {
                                ui.label(egui::RichText::new("(new)").size(10.0).weak().italics());
                            }
                        });
                    }
                    ui.add_space(4.0);
                } else {
                    // Fallback to showing current map indicator
                    if let Some(ref current_path) = map_res.current_map_file.path {
                        let current_name = current_path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        let dirty_indicator = if map_res.dirty_state.is_dirty { "*" } else { "" };
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Current:").size(12.0).weak());
                            ui.label(
                                egui::RichText::new(format!("{}{}", current_name, dirty_indicator))
                                    .size(12.0)
                                    .strong(),
                            );
                        });
                        ui.add_space(4.0);
                    } else {
                        let dirty_indicator = if map_res.dirty_state.is_dirty { "*" } else { "" };
                        ui.label(
                            egui::RichText::new(format!("(unsaved map){}", dirty_indicator))
                                .size(12.0)
                                .weak()
                                .italics(),
                        );
                        ui.add_space(4.0);
                    }
                }

                // Switch to selected map if requested
                if let Some(target_id) = map_to_switch {
                    map_res.switch_events.write(SwitchMapRequest { map_id: target_id });
                }

                ui.horizontal(|ui| {
                    if ui.add_sized([45.0, 24.0], egui::Button::new("New")).clicked() {
                        dialogs.menu_state.show_new_confirmation = true;
                    }
                    if ui.add_sized([45.0, 24.0], egui::Button::new("Save")).clicked() {
                        dialogs.menu_state.save_filename = map_res.map_data.name.clone();
                        dialogs.menu_state.show_save_name_dialog = true;
                    }
                    if ui.add_sized([55.0, 24.0], egui::Button::new("Rename"))
                        .on_hover_text("Rename map (F3)")
                        .clicked()
                    {
                        browser_state.rename_map_new_name = map_res.map_data.name.clone();
                        browser_state.rename_map_dialog_open = true;
                    }
                });

                // Scan maps directory (within library only)
                let maps_dir = library.library_path.join("maps");

                // Ensure maps directory exists
                if !maps_dir.exists()
                    && let Err(e) = std::fs::create_dir_all(&maps_dir)
                {
                    warn!("Failed to create maps directory: {}", e);
                }

                // Refresh cached maps if directory changed
                if browser_state.last_maps_scan_path.as_ref() != Some(&maps_dir) {
                    browser_state.cached_maps = scan_maps_directory(&maps_dir);
                    browser_state.last_maps_scan_path = Some(maps_dir.clone());
                }

                if !browser_state.cached_maps.is_empty() {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Available:").size(12.0).weak());
                        if ui.small_button("R").on_hover_text("Refresh map list").clicked() {
                            browser_state.cached_maps = scan_maps_directory(&maps_dir);
                        }
                    });

                    // Show map buttons in a scrollable area
                    egui::ScrollArea::vertical()
                        .id_salt("maps_scroll")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for (map_name, map_path) in &browser_state.cached_maps {
                                let is_current = map_res.current_map_file
                                    .path
                                    .as_ref()
                                    .map(|p| p == map_path)
                                    .unwrap_or(false);

                                ui.horizontal(|ui| {
                                    if is_current {
                                        ui.label(egui::RichText::new(">").size(10.0));
                                    } else {
                                        ui.add_space(12.0);
                                    }

                                    let button_text = if is_current {
                                        egui::RichText::new(map_name).size(12.0).strong()
                                    } else {
                                        egui::RichText::new(map_name).size(12.0)
                                    };

                                    if ui
                                        .add(egui::Button::new(button_text).frame(false))
                                        .on_hover_text(map_path.to_string_lossy().as_ref())
                                        .clicked()
                                        && !is_current
                                    {
                                        map_res.load_events.write(LoadMapRequest {
                                            path: map_path.clone(),
                                        });
                                    }
                                });
                            }
                        });
                }

                ui.add_space(10.0);

                // Assets subsection
                ui.label(egui::RichText::new("Assets").size(13.0).strong());
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.add_sized([80.0, 24.0], egui::Button::new("Import...")).clicked() {
                        dialogs.import_dialog.is_open = true;
                    }
                    if ui.add_sized([80.0, 24.0], egui::Button::new("Open Folder")).on_hover_text("Open library folder in file explorer").clicked() {
                        let path = &library.library_path;
                        #[cfg(target_os = "linux")]
                        {
                            let _ = std::process::Command::new("xdg-open")
                                .arg(path)
                                .spawn();
                        }
                        #[cfg(target_os = "macos")]
                        {
                            let _ = std::process::Command::new("open")
                                .arg(path)
                                .spawn();
                        }
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("explorer")
                                .arg(path)
                                .spawn();
                        }
                    }
                });

                ui.add_space(6.0);
            }

            ui.separator();
            ui.add_space(4.0);

            // Update discovered folders when library changes
            if browser_state.last_library_path.as_ref() != Some(&library.library_path) {
                browser_state.discovered_folders = discover_folders(&library);
            }

            // Folder tree view
            ui.label(egui::RichText::new("Folders").size(12.0).weak());
            egui::ScrollArea::vertical()
                .id_salt("folder_tree")
                .max_height(120.0)
                .show(ui, |ui| {
                    // Root folder (library root)
                    let root_selected = browser_state.selected_folder.is_empty();
                    if ui
                        .selectable_label(root_selected, egui::RichText::new("(root)").size(12.0))
                        .clicked()
                    {
                        browser_state.selected_folder = String::new();
                    }

                    // Render folders as a tree
                    for folder in &browser_state.discovered_folders.clone() {
                        let is_selected = browser_state.selected_folder == *folder;
                        let depth = folder.matches('/').count();
                        let display_name = folder.split('/').next_back().unwrap_or(folder);

                        ui.horizontal(|ui| {
                            ui.add_space(depth as f32 * 12.0);
                            if ui
                                .selectable_label(is_selected, egui::RichText::new(display_name).size(12.0))
                                .clicked()
                            {
                                browser_state.selected_folder = folder.clone();
                            }
                        });
                    }
                });

            ui.add_space(4.0);
            ui.separator();

            let filtered_assets: Vec<&LibraryAsset> = library
                .assets
                .iter()
                .filter(|a| a.folder_path == browser_state.selected_folder)
                .collect();

            if filtered_assets.is_empty() {
                ui.label("No assets in this folder.");
                let folder_display = if browser_state.selected_folder.is_empty() {
                    library.library_path.display().to_string()
                } else {
                    library.library_path.join(&browser_state.selected_folder).display().to_string()
                };
                ui.label(format!("Add images to {}/", folder_display));
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for asset in filtered_assets {
                        let is_selected = selected_asset
                            .asset
                            .as_ref()
                            .map(|a| a.relative_path == asset.relative_path)
                            .unwrap_or(false);

                        // Asset row with thumbnail preview
                        ui.horizontal(|ui| {
                            let thumb_size = THUMBNAIL_SIZE as f32;

                            // Try to get cached thumbnail texture ID
                            if let Some(texture_id) =
                                thumbnail_cache.get_texture_id(&asset.full_path)
                            {
                                ui.add(
                                    egui::Image::new(egui::load::SizedTexture::new(
                                        texture_id,
                                        egui::vec2(thumb_size, thumb_size),
                                    ))
                                    .fit_to_exact_size(egui::vec2(thumb_size, thumb_size))
                                    .corner_radius(2.0),
                                );
                            } else {
                                // Placeholder while loading
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(thumb_size, thumb_size),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    2.0,
                                    egui::Color32::from_rgb(60, 60, 60),
                                );
                            }

                            // Check if asset file is missing
                            let is_missing = thumbnail_cache.has_failed(&asset.full_path)
                                || !asset.full_path.exists();

                            // Asset name (selectable) with visual indicator if missing
                            let label_text = if is_missing {
                                egui::RichText::new(&asset.name)
                                    .color(egui::Color32::from_rgb(200, 100, 100))
                            } else {
                                egui::RichText::new(&asset.name)
                            };

                            let response = ui.selectable_label(is_selected, label_text);

                            // Apply hover text and get final response
                            let response = if is_missing {
                                response.on_hover_text("Asset file not found")
                            } else {
                                response
                            };

                            if response.clicked() {
                                // Validate file exists before selecting
                                if !asset.full_path.exists() {
                                    // Mark as failed in thumbnail cache
                                    thumbnail_cache.failed.insert(asset.full_path.clone(), ());
                                    warn!(
                                        "Selected asset no longer exists: {:?}",
                                        asset.full_path
                                    );
                                } else {
                                    browser_state.selected_dimensions = None;
                                    browser_state.cached_dimensions_path = None;
                                    selected_asset.asset = Some(asset.clone());
                                    current_tool.tool = EditorTool::Place;
                                }
                            }

                            // File type badge on the right
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let ext_short = asset
                                        .extension
                                        .chars()
                                        .take(3)
                                        .collect::<String>()
                                        .to_uppercase();
                                    ui.label(
                                        egui::RichText::new(ext_short)
                                            .small()
                                            .color(extension_color(&asset.extension)),
                                    );
                                },
                            );
                        });
                    }
                });
            }

            ui.separator();
            ui.add_space(4.0);

            // =========================================
            // SELECTED ASSET METADATA SECTION
            // =========================================
            if let Some(ref asset) = selected_asset.asset {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Selected Asset").size(14.0).strong());
                    // Rename button
                    if ui.small_button("Rename").on_hover_text("Rename asset (F2)").clicked() {
                        browser_state.rename_new_name = asset.name.clone();
                        browser_state.rename_error = None;
                        browser_state.rename_dialog_open = true;
                    }
                    // Move button
                    if ui.small_button("Move").on_hover_text("Move to another category").clicked() {
                        browser_state.move_error = None;
                        browser_state.move_dialog_open = true;
                    }
                });
                ui.add_space(6.0);

                // Asset name
                ui.label(egui::RichText::new(&asset.name).size(13.0));

                ui.add_space(6.0);

                // Metadata in a subtle style
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Type:").size(13.0).weak());
                    ui.label(
                        egui::RichText::new(asset.extension.to_uppercase())
                            .size(13.0)
                            .strong(),
                    );
                });

                // Get/cache dimensions
                let needs_dimension_load = browser_state
                    .cached_dimensions_path
                    .as_ref()
                    .map(|p| p != &asset.full_path)
                    .unwrap_or(true);

                if needs_dimension_load {
                    browser_state.selected_dimensions = get_image_dimensions(&asset.full_path);
                    browser_state.cached_dimensions_path = Some(asset.full_path.clone());
                }

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Size:").size(13.0).weak());
                    if let Some((width, height)) = browser_state.selected_dimensions {
                        ui.label(
                            egui::RichText::new(format!("{}x{}", width, height))
                                .size(13.0)
                                .strong(),
                        );
                    } else {
                        ui.label(egui::RichText::new("Unknown").size(13.0).weak());
                    }
                });
            } else {
                ui.label(egui::RichText::new("No asset selected").size(13.0).weak());
            }

            // Settings button at bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                if ui
                    .add_sized([100.0, 24.0], egui::Button::new("Settings"))
                    .clicked()
                {
                    dialogs.settings_state.load_from_config(&config);
                    dialogs.settings_state.load_library_info(&library);
                    dialogs.settings_state.is_open = true;
                }
                ui.add_space(4.0);
                ui.separator();
            });
        });

    // "Set as default library" dialog
    if browser_state.show_set_default_dialog {
        egui::Window::new("Library Opened")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                if let Some(ref path) = browser_state.set_default_dialog_path {
                    let display_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown");
                    ui.label(format!("Opened library: {}", display_name));
                    ui.add_space(8.0);
                    ui.checkbox(
                        &mut browser_state.set_as_default_checked,
                        "Set as default library",
                    );
                    ui.add_space(8.0);
                }

                if ui.button("OK").clicked() {
                    if browser_state.set_as_default_checked
                        && let Some(ref path) = browser_state.set_default_dialog_path
                    {
                        set_default_events.write(SetDefaultLibraryRequest { path: path.clone() });
                    }
                    browser_state.show_set_default_dialog = false;
                    browser_state.set_default_dialog_path = None;
                    browser_state.set_as_default_checked = false;
                }
            });
    }

    // Library import error dialog
    if let Some(ref error) = browser_state.library_import_error.clone() {
        egui::Window::new("Import Error")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                ui.colored_label(egui::Color32::RED, "Failed to import library");
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    ui.label(error);
                });
                ui.add_space(8.0);
                if ui.button("OK").clicked() {
                    browser_state.library_import_error = None;
                }
            });
    }

    // Library operation success dialog
    if let Some(ref message) = browser_state.library_operation_success.clone() {
        egui::Window::new("Success")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                ui.label(message);
                ui.add_space(8.0);
                if ui.button("OK").clicked() {
                    browser_state.library_operation_success = None;
                }
            });
    }

    // Asset rename dialog
    if browser_state.rename_dialog_open {
        let mut close_dialog = false;
        let mut do_rename = false;

        egui::Window::new("Rename Asset")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                ui.label("Enter new name:");
                ui.add_space(4.0);

                let response = ui.text_edit_singleline(&mut browser_state.rename_new_name);

                // Request focus only when first opened (not every frame)
                if !response.has_focus() {
                    response.request_focus();
                }

                // Handle Enter key
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    do_rename = true;
                }

                // Show error message if any
                if let Some(ref error) = browser_state.rename_error {
                    ui.add_space(4.0);
                    ui.colored_label(egui::Color32::RED, error);
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        do_rename = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });

        // Handle rename action
        if do_rename
            && let Some(ref asset) = selected_asset.asset.clone()
        {
            match rename_asset(&asset.full_path, &browser_state.rename_new_name, &library.library_path) {
                Ok((new_path, old_relative, new_relative)) => {
                    // Update the asset in the library
                    if let Some(lib_asset) = library.assets.iter_mut().find(|a| a.full_path == asset.full_path) {
                        lib_asset.full_path = new_path.clone();
                        lib_asset.relative_path = new_relative.clone();
                        lib_asset.name = browser_state.rename_new_name.trim().to_string();
                    }

                    // Update the selected asset
                    if let Some(ref mut sel) = selected_asset.asset {
                        sel.full_path = new_path;
                        sel.relative_path = new_relative.clone();
                        sel.name = browser_state.rename_new_name.trim().to_string();
                    }

                    // Clear thumbnail cache for this asset (path changed)
                    thumbnail_cache.thumbnails.remove(&asset.full_path);
                    thumbnail_cache.texture_ids.remove(&asset.full_path);

                    // Emit message to update currently placed items
                    rename_events.write(RenameAssetRequest {
                        old_path: old_relative,
                        new_path: new_relative,
                    });

                    browser_state.rename_dialog_open = false;
                    browser_state.rename_error = None;
                    info!("Renamed asset: {} -> {}", asset.name, browser_state.rename_new_name);
                }
                Err(e) => {
                    browser_state.rename_error = Some(e);
                }
            }
        }

        if close_dialog {
            browser_state.rename_dialog_open = false;
            browser_state.rename_error = None;
        }
    }

    // Rename map dialog
    if browser_state.rename_map_dialog_open {
        let mut close_dialog = false;
        let mut do_rename = false;

        egui::Window::new("Rename Map")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                ui.label("Enter new map name:");
                ui.add_space(4.0);

                let response = ui.text_edit_singleline(&mut browser_state.rename_map_new_name);

                // Request focus only when first opened (not every frame)
                if !response.has_focus() {
                    response.request_focus();
                }

                // Handle Enter key
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    do_rename = true;
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        do_rename = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });

        if do_rename {
            let new_name = browser_state.rename_map_new_name.trim().to_string();
            if !new_name.is_empty() {
                map_res.map_data.name = new_name;
                map_res.dirty_state.is_dirty = true;
                browser_state.rename_map_dialog_open = false;
            }
        }

        if close_dialog {
            browser_state.rename_map_dialog_open = false;
        }
    }

    // Rename library dialog
    if browser_state.rename_library_dialog_open {
        let mut close_dialog = false;
        let mut do_rename = false;

        egui::Window::new("Rename Library")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                ui.label("Enter new library name:");
                ui.add_space(4.0);

                let response = ui.text_edit_singleline(&mut browser_state.rename_library_new_name);

                // Request focus only when first opened (not every frame)
                if !response.has_focus() {
                    response.request_focus();
                }

                // Handle Enter key
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    do_rename = true;
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Rename").clicked() {
                        do_rename = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });

        if do_rename {
            let new_name = browser_state.rename_library_new_name.trim().to_string();
            if !new_name.is_empty() {
                library_metadata_events.write(UpdateLibraryMetadataRequest { name: new_name });
                browser_state.rename_library_dialog_open = false;
            }
        }

        if close_dialog {
            browser_state.rename_library_dialog_open = false;
        }
    }

    // Move asset dialog
    if browser_state.move_dialog_open {
        let mut close_dialog = false;
        let mut target_folder: Option<String> = None;

        egui::Window::new("Move Asset")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                if let Some(ref asset) = selected_asset.asset {
                    ui.label(format!("Move '{}' to:", asset.name));
                    ui.add_space(8.0);

                    // Root folder option
                    if !asset.folder_path.is_empty() && ui.button("(root)").clicked() {
                        target_folder = Some(String::new());
                    }

                    // Existing folder options (excluding current folder)
                    for folder in &browser_state.discovered_folders.clone() {
                        if *folder != asset.folder_path && ui.button(folder).clicked() {
                            target_folder = Some(folder.clone());
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.label("Or create new folder:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut browser_state.move_new_folder_name);
                        if ui.button("Create & Move").clicked()
                            && !browser_state.move_new_folder_name.is_empty()
                        {
                            target_folder = Some(browser_state.move_new_folder_name.clone());
                        }
                    });

                    // Show error message if any
                    if let Some(ref error) = browser_state.move_error {
                        ui.add_space(4.0);
                        ui.colored_label(egui::Color32::RED, error);
                    }

                    ui.add_space(8.0);
                    if ui.button("Cancel").clicked() {
                        close_dialog = true;
                    }
                } else {
                    ui.label("No asset selected");
                    if ui.button("Close").clicked() {
                        close_dialog = true;
                    }
                }
            });

        // Handle move action
        if let Some(folder) = target_folder
            && let Some(ref asset) = selected_asset.asset.clone()
        {
            match move_asset(&asset.full_path, &folder, &library.library_path) {
                Ok((new_path, old_relative, new_relative)) => {
                    // Update the asset in the library
                    if let Some(lib_asset) = library.assets.iter_mut().find(|a| a.full_path == asset.full_path) {
                        lib_asset.full_path = new_path.clone();
                        lib_asset.relative_path = new_relative.clone();
                        lib_asset.folder_path = folder.clone();
                    }

                    // Update the selected asset
                    if let Some(ref mut sel) = selected_asset.asset {
                        sel.full_path = new_path;
                        sel.relative_path = new_relative.clone();
                        sel.folder_path = folder.clone();
                    }

                    // Update the browser to show the new folder
                    browser_state.selected_folder = folder.clone();
                    // Refresh discovered folders
                    browser_state.discovered_folders = discover_folders(&library);

                    // Clear thumbnail cache for this asset (path changed)
                    thumbnail_cache.thumbnails.remove(&asset.full_path);
                    thumbnail_cache.texture_ids.remove(&asset.full_path);

                    // Emit message to update currently placed items
                    rename_events.write(RenameAssetRequest {
                        old_path: old_relative,
                        new_path: new_relative,
                    });

                    browser_state.move_dialog_open = false;
                    browser_state.move_error = None;
                    browser_state.move_new_folder_name.clear();
                    let folder_display = if folder.is_empty() { "(root)" } else { &folder };
                    info!("Moved asset '{}' to {}", asset.name, folder_display);
                }
                Err(e) => {
                    browser_state.move_error = Some(e);
                }
            }
        }

        if close_dialog {
            browser_state.move_dialog_open = false;
            browser_state.move_error = None;
            browser_state.move_new_folder_name.clear();
        }
    }

    Ok(())
}

/// Get a color for the preview square based on file extension
fn extension_color(ext: &str) -> egui::Color32 {
    match ext {
        "png" => egui::Color32::from_rgb(80, 140, 200),   // Blue
        "jpg" | "jpeg" => egui::Color32::from_rgb(200, 140, 80), // Orange
        "webp" => egui::Color32::from_rgb(140, 200, 80),  // Green
        "gif" => egui::Color32::from_rgb(200, 80, 140),   // Pink
        "bmp" => egui::Color32::from_rgb(140, 80, 200),   // Purple
        "tiff" | "tif" => egui::Color32::from_rgb(80, 200, 140), // Teal
        _ => egui::Color32::from_rgb(128, 128, 128),      // Gray
    }
}
