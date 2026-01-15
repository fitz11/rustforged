use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::RefreshAssetLibrary;
use crate::config::AddRecentLibraryRequest;

/// Size of thumbnail previews in pixels
pub const THUMBNAIL_SIZE: u32 = 24;

/// Filename for library metadata (hidden file)
pub const LIBRARY_METADATA_FILE: &str = ".library.json";

/// Metadata for an asset library, stored in .library.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryMetadata {
    /// User-defined name for the library
    pub name: String,
}

impl Default for LibraryMetadata {
    fn default() -> Self {
        Self {
            name: "Unnamed Library".to_string(),
        }
    }
}

#[derive(Resource)]
pub struct AssetLibrary {
    pub library_path: PathBuf,
    pub assets: Vec<LibraryAsset>,
    /// Error message if the last library operation failed
    pub error: Option<String>,
    /// Library metadata (name, etc.)
    pub metadata: LibraryMetadata,
}

impl Default for AssetLibrary {
    fn default() -> Self {
        Self {
            library_path: crate::paths::default_library_dir(),
            assets: Vec::new(),
            error: None,
            metadata: LibraryMetadata::default(),
        }
    }
}

/// Result of validating/opening an asset library directory
#[derive(Debug)]
pub enum LibraryValidation {
    /// Directory is a valid asset library
    Valid,
    /// Directory doesn't exist
    NotFound,
    /// Other error (permissions, etc.)
    Error(String),
}

/// Validates that a directory can be used as an asset library
pub fn validate_library_directory(path: &Path) -> LibraryValidation {
    if !path.exists() {
        return LibraryValidation::NotFound;
    }

    if !path.is_dir() {
        return LibraryValidation::Error("Path is not a directory".to_string());
    }

    LibraryValidation::Valid
}

/// Creates a new asset library directory with suggested subfolders
pub fn create_library_directory(path: &Path) -> Result<(), String> {
    // Create the main directory if it doesn't exist
    if !path.exists() {
        std::fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    // Create suggested asset folders (terrain, doodads, tokens)
    for folder in ["terrain", "doodads", "tokens"] {
        let subfolder = path.join(folder);
        if !subfolder.exists() {
            std::fs::create_dir_all(&subfolder)
                .map_err(|e| format!("Failed to create {} folder: {}", folder, e))?;
        }
    }

    // Create maps folder
    let maps_folder = path.join("maps");
    if !maps_folder.exists() {
        std::fs::create_dir_all(&maps_folder)
            .map_err(|e| format!("Failed to create maps folder: {}", e))?;
    }

    Ok(())
}

/// Load library metadata from .library.json file
pub fn load_library_metadata(library_path: &Path) -> LibraryMetadata {
    let metadata_path = library_path.join(LIBRARY_METADATA_FILE);

    if metadata_path.exists() {
        match std::fs::read_to_string(&metadata_path) {
            Ok(json) => match serde_json::from_str(&json) {
                Ok(metadata) => return metadata,
                Err(e) => warn!("Failed to parse library metadata: {}", e),
            },
            Err(e) => warn!("Failed to read library metadata: {}", e),
        }
    }

    // Return default with name derived from directory
    let name = library_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unnamed Library")
        .to_string();

    LibraryMetadata { name }
}

/// Save library metadata to .library.json file
pub fn save_library_metadata(library_path: &Path, metadata: &LibraryMetadata) -> Result<(), String> {
    let metadata_path = library_path.join(LIBRARY_METADATA_FILE);

    let json = serde_json::to_string_pretty(metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

    std::fs::write(&metadata_path, json)
        .map_err(|e| format!("Failed to write metadata file: {}", e))?;

    info!("Saved library metadata to {:?}", metadata_path);
    Ok(())
}

#[derive(Debug, Clone)]
pub struct LibraryAsset {
    pub name: String,
    /// Full relative path for asset loading (e.g., "library/terrain/stone/floor.png")
    pub relative_path: String,
    /// Parent folder relative path from library root (e.g., "terrain/stone" or "" for root)
    pub folder_path: String,
    /// File extension (e.g., "png", "jpg")
    pub extension: String,
    /// Full filesystem path for reading metadata
    pub full_path: PathBuf,
}

/// Scans assets from a library directory into the AssetLibrary resource
fn scan_library_at_path(library: &mut AssetLibrary, library_path: &Path) {
    library.assets.clear();
    library.error = None;

    scan_directory_recursive(library, library_path, library_path);
}

/// Recursively scans a directory for image assets
fn scan_directory_recursive(library: &mut AssetLibrary, base_path: &Path, current_path: &Path) {
    let entries = match std::fs::read_dir(current_path) {
        Ok(e) => e,
        Err(e) => {
            warn!(
                "Cannot read directory {:?}: {}. Some assets may not be loaded.",
                current_path, e
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip hidden files/directories (starting with .)
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        if path.is_dir() {
            // Skip the "maps" directory
            if path.file_name().and_then(|n| n.to_str()) == Some("maps") {
                continue;
            }
            // Recurse into subdirectories
            scan_directory_recursive(library, base_path, &path);
            continue;
        }

        if !is_image_file(&path) {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Calculate folder path relative to library root
        let folder_path = current_path
            .strip_prefix(base_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // For assets in the default library location, use relative path for Bevy asset loading
        // For external libraries, use absolute path
        let relative_path = if base_path.starts_with("assets/library") {
            let asset_relative = path
                .strip_prefix(base_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string());
            format!("library/{}", asset_relative)
        } else {
            // Use absolute path for external libraries
            path.to_string_lossy().to_string()
        };

        library.assets.push(LibraryAsset {
            name,
            relative_path,
            folder_path,
            extension,
            full_path: path,
        });
    }
}

/// Initial scan from default asset library location
pub fn scan_asset_library(mut library: ResMut<AssetLibrary>) {
    let library_path = library.library_path.clone();

    // Create default library structure if it doesn't exist
    if !library_path.exists()
        && let Err(e) = crate::paths::setup_default_library()
    {
        warn!("Failed to create default library: {}", e);
    }

    // Load library metadata
    library.metadata = load_library_metadata(&library_path);

    // Ensure metadata file exists (create if missing for migration)
    if !library_path.join(LIBRARY_METADATA_FILE).exists()
        && let Err(e) = save_library_metadata(&library_path, &library.metadata)
    {
        warn!("Failed to create metadata file: {}", e);
    }

    // Ensure maps folder exists
    let maps_folder = library_path.join("maps");
    if !maps_folder.exists()
        && let Err(e) = std::fs::create_dir_all(&maps_folder)
    {
        warn!("Failed to create maps folder: {}", e);
    }

    scan_library_at_path(&mut library, &library_path);
    info!(
        "Loaded {} assets from library '{}'",
        library.assets.len(),
        library.metadata.name
    );
}

fn is_image_file(path: &Path) -> bool {
    let extensions = ["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif"];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Get the dimensions of an image file (width, height)
pub fn get_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    // Use the image crate to read just the header for dimensions
    image::image_dimensions(path).ok()
}

/// Cache for asset thumbnail textures
#[derive(Resource, Default)]
pub struct ThumbnailCache {
    /// Maps asset full_path to the loaded Bevy image handle
    pub thumbnails: HashMap<PathBuf, Handle<Image>>,
    /// Maps asset full_path to the egui texture ID (set after registration)
    pub texture_ids: HashMap<PathBuf, bevy_egui::egui::TextureId>,
    /// Tracks paths that failed to load (so we don't retry)
    pub failed: HashMap<PathBuf, ()>,
}

impl ThumbnailCache {
    /// Get the egui texture ID if already registered
    pub fn get_texture_id(&self, path: &Path) -> Option<bevy_egui::egui::TextureId> {
        self.texture_ids.get(path).copied()
    }

    /// Check if loading this thumbnail previously failed
    pub fn has_failed(&self, path: &Path) -> bool {
        self.failed.contains_key(path)
    }

    /// Clear the cache (call when switching libraries)
    pub fn clear(&mut self) {
        self.thumbnails.clear();
        self.texture_ids.clear();
        self.failed.clear();
    }
}

/// Load a thumbnail image from a file path, returning a Bevy Image.
/// For animated formats (GIF), only the first frame is loaded.
/// The image is resized to THUMBNAIL_SIZE x THUMBNAIL_SIZE.
pub fn load_thumbnail(path: &Path) -> Option<Image> {
    use image::imageops::FilterType;
    use image::{GenericImageView, ImageReader};

    // Open and decode the image (for GIFs, this gets the first frame)
    let img = ImageReader::open(path).ok()?.decode().ok()?;

    // Calculate aspect-preserving resize dimensions
    let (orig_w, orig_h) = img.dimensions();
    let (thumb_w, thumb_h) = if orig_w > orig_h {
        (
            THUMBNAIL_SIZE,
            (THUMBNAIL_SIZE as f32 * orig_h as f32 / orig_w as f32).max(1.0) as u32,
        )
    } else {
        (
            (THUMBNAIL_SIZE as f32 * orig_w as f32 / orig_h as f32).max(1.0) as u32,
            THUMBNAIL_SIZE,
        )
    };

    // Resize the image
    let resized = img.resize(thumb_w, thumb_h, FilterType::Triangle);

    // Convert to RGBA8
    let rgba = resized.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Create Bevy Image
    Some(Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        default(),
    ))
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

/// Opens an existing asset library directory
pub fn open_library_directory(library: &mut AssetLibrary, path: PathBuf) -> Result<(), String> {
    match validate_library_directory(&path) {
        LibraryValidation::Valid => {
            library.library_path = path.clone();
            library.metadata = load_library_metadata(&path);

            // Ensure metadata file exists (create if missing for migration)
            if !path.join(LIBRARY_METADATA_FILE).exists()
                && let Err(e) = save_library_metadata(&path, &library.metadata)
            {
                warn!("Failed to create metadata file: {}", e);
            }

            // Ensure maps folder exists
            let maps_folder = path.join("maps");
            if !maps_folder.exists()
                && let Err(e) = std::fs::create_dir_all(&maps_folder)
            {
                warn!("Failed to create maps folder: {}", e);
            }

            scan_library_at_path(library, &path);
            library.error = None;
            info!(
                "Opened asset library '{}' at {:?} with {} assets",
                library.metadata.name,
                path,
                library.assets.len()
            );
            Ok(())
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

    // Create metadata with name from folder
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unnamed Library")
        .to_string();

    library.metadata = LibraryMetadata { name };
    save_library_metadata(&path, &library.metadata)?;

    library.library_path = path.clone();
    scan_library_at_path(library, &path);
    library.error = None;
    info!(
        "Created and opened new asset library '{}' at {:?}",
        library.metadata.name, path
    );
    Ok(())
}

/// Track library changes and add to recent libraries list
pub fn track_library_changes(
    library: Res<AssetLibrary>,
    mut last_path: Local<Option<PathBuf>>,
    mut recent_events: MessageWriter<AddRecentLibraryRequest>,
) {
    // Skip if library path hasn't changed
    if last_path.as_ref() == Some(&library.library_path) {
        return;
    }

    // Only add to recent if the library opened successfully (no error)
    if library.error.is_none() {
        recent_events.write(AddRecentLibraryRequest {
            path: library.library_path.clone(),
        });
    }

    *last_path = Some(library.library_path.clone());
}
