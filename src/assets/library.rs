use bevy::prelude::*;
use std::path::{Path, PathBuf};

use super::{AssetCategory, RefreshAssetLibrary};

#[derive(Resource)]
pub struct AssetLibrary {
    pub library_path: PathBuf,
    pub assets: Vec<LibraryAsset>,
    /// Error message if the last library operation failed
    pub error: Option<String>,
}

impl Default for AssetLibrary {
    fn default() -> Self {
        Self {
            library_path: PathBuf::from("assets/library"),
            assets: Vec::new(),
            error: None,
        }
    }
}

/// Result of validating/opening an asset library directory
#[derive(Debug)]
pub enum LibraryValidation {
    /// Directory is a valid asset library with all required subfolders
    Valid,
    /// Directory exists but is missing required subfolders
    MissingFolders(Vec<String>),
    /// Directory doesn't exist
    NotFound,
    /// Other error (permissions, etc.)
    Error(String),
}

/// Validates that a directory is a valid asset library (has required subfolders)
pub fn validate_library_directory(path: &Path) -> LibraryValidation {
    if !path.exists() {
        return LibraryValidation::NotFound;
    }

    if !path.is_dir() {
        return LibraryValidation::Error("Path is not a directory".to_string());
    }

    let mut missing = Vec::new();
    for category in AssetCategory::all() {
        let subfolder = path.join(category.folder_name());
        if !subfolder.exists() {
            missing.push(category.folder_name().to_string());
        }
    }

    if missing.is_empty() {
        LibraryValidation::Valid
    } else {
        LibraryValidation::MissingFolders(missing)
    }
}

/// Creates a new asset library directory with all required subfolders
pub fn create_library_directory(path: &Path) -> Result<(), String> {
    // Create the main directory if it doesn't exist
    if !path.exists() {
        std::fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    // Create all category subfolders
    for category in AssetCategory::all() {
        let subfolder = path.join(category.folder_name());
        if !subfolder.exists() {
            std::fs::create_dir_all(&subfolder)
                .map_err(|e| format!("Failed to create {} folder: {}", category.folder_name(), e))?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct LibraryAsset {
    pub name: String,
    pub relative_path: String,
    pub category: AssetCategory,
}

/// Scans assets from a library directory into the AssetLibrary resource
fn scan_library_at_path(library: &mut AssetLibrary, library_path: &Path) {
    library.assets.clear();
    library.error = None;

    for category in AssetCategory::all() {
        let category_path = library_path.join(category.folder_name());

        if !category_path.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(&category_path) {
            for entry in entries.flatten() {
                let path = entry.path();

                if !is_image_file(&path) {
                    continue;
                }

                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                // For assets outside the default location, store absolute path
                let relative_path = if library_path.starts_with("assets/library") {
                    format!(
                        "library/{}/{}",
                        category.folder_name(),
                        path.file_name().unwrap().to_str().unwrap()
                    )
                } else {
                    // Use absolute path for external libraries
                    path.to_string_lossy().to_string()
                };

                library.assets.push(LibraryAsset {
                    name,
                    relative_path,
                    category: *category,
                });
            }
        }
    }
}

/// Initial scan from default asset library location
pub fn scan_asset_library(mut library: ResMut<AssetLibrary>) {
    let library_path = library.library_path.clone();

    // Create default library structure if it doesn't exist
    if library_path == Path::new("assets/library")
        && let Err(e) = create_library_directory(&library_path)
    {
        warn!("Failed to create default library: {}", e);
    }

    scan_library_at_path(&mut library, &library_path);
    info!("Loaded {} assets from library", library.assets.len());
}

fn is_image_file(path: &Path) -> bool {
    let extensions = ["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif"];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn refresh_asset_library(
    mut events: MessageReader<RefreshAssetLibrary>,
    mut library: ResMut<AssetLibrary>,
) {
    for _ in events.read() {
        let library_path = library.library_path.clone();
        scan_library_at_path(&mut library, &library_path);
        info!("Refreshed asset library: {} assets", library.assets.len());
    }
}

/// Opens an existing asset library directory, validating it has required subfolders
pub fn open_library_directory(library: &mut AssetLibrary, path: PathBuf) -> Result<(), String> {
    match validate_library_directory(&path) {
        LibraryValidation::Valid => {
            library.library_path = path.clone();
            scan_library_at_path(library, &path);
            library.error = None;
            info!("Opened asset library at {:?} with {} assets", path, library.assets.len());
            Ok(())
        }
        LibraryValidation::MissingFolders(missing) => {
            let err = format!(
                "Directory is missing required folders: {}",
                missing.join(", ")
            );
            library.error = Some(err.clone());
            Err(err)
        }
        LibraryValidation::NotFound => {
            let err = "Directory not found".to_string();
            library.error = Some(err.clone());
            Err(err)
        }
        LibraryValidation::Error(e) => {
            library.error = Some(e.clone());
            Err(e)
        }
    }
}

/// Creates a new asset library directory with required subfolders and opens it
pub fn create_and_open_library(library: &mut AssetLibrary, path: PathBuf) -> Result<(), String> {
    create_library_directory(&path)?;
    library.library_path = path.clone();
    scan_library_at_path(library, &path);
    library.error = None;
    info!("Created and opened new asset library at {:?}", path);
    Ok(())
}
