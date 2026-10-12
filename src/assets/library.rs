use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::EguiUserTextures;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::RefreshAssetLibrary;
use super::fingerprint::{FileStamp, FingerprintCache, LibraryIndex};
use super::manifest::{
    LibraryMetadata, load_library_metadata, new_library_metadata, save_library_metadata,
};
use super::source::{LibraryRoot, library_asset_path};
use crate::config::AddRecentLibraryRequest;

/// Size of thumbnail previews in pixels
pub const THUMBNAIL_SIZE: u32 = 24;

#[derive(Resource)]
pub struct AssetLibrary {
    /// Root directory of the open library. Change it with [`AssetLibrary::set_library_path`]
    /// so the `lib://` asset source follows.
    pub library_path: PathBuf,
    pub assets: Vec<LibraryAsset>,
    /// Error message if the last library operation failed
    pub error: Option<String>,
    /// Library metadata (name, etc.)
    pub metadata: LibraryMetadata,
    /// Root the `lib://` asset source resolves against
    root: LibraryRoot,
    /// Bumped on every scan, so systems can react to rescans without relying on
    /// change detection (the browser UI touches this resource mutably every frame).
    generation: u64,
    /// Content fingerprints of this library's files, filled lazily by save/load tasks
    pub fingerprints: FingerprintCache,
    /// Bumped when save/load tasks add fingerprints, so the cache knows to persist them
    fingerprint_revision: u64,
}

impl Default for AssetLibrary {
    fn default() -> Self {
        Self::with_root(LibraryRoot::default())
    }
}

impl AssetLibrary {
    /// Creates the library resource around the `lib://` source's shared root.
    pub fn with_root(root: LibraryRoot) -> Self {
        let library_path = crate::paths::default_library_dir();
        root.set(library_path.clone());
        Self {
            library_path,
            assets: Vec::new(),
            error: None,
            metadata: LibraryMetadata::default(),
            root,
            generation: 0,
            fingerprints: FingerprintCache::default(),
            fingerprint_revision: 0,
        }
    }

    /// Increases every time the asset list is rescanned.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Points the library, and the `lib://` asset source, at a new directory.
    pub fn set_library_path(&mut self, path: PathBuf) {
        if path != self.library_path {
            self.fingerprints = FingerprintCache::default();
        }
        self.root.set(path.clone());
        self.library_path = path;
    }

    /// Adds fingerprints computed by a background task.
    pub fn merge_fingerprints(&mut self, computed: FingerprintCache) {
        if !computed.is_empty() {
            self.fingerprints.merge(computed);
            self.fingerprint_revision += 1;
        }
    }

    /// Adds fingerprints saved by an earlier session (they don't need saving again).
    pub fn load_saved_fingerprints(&mut self, saved: FingerprintCache) {
        let fresh = std::mem::replace(&mut self.fingerprints, saved);
        self.fingerprints.merge(fresh);
    }

    /// Increases whenever fingerprints worth saving are added.
    pub fn fingerprint_revision(&self) -> u64 {
        self.fingerprint_revision
    }

    /// Snapshot for fingerprinting and resolving references off the main thread.
    pub fn index(&self) -> LibraryIndex {
        LibraryIndex::new(
            self.library_path.clone(),
            self.assets.iter().map(|a| (a.path.clone(), a.stamp)),
            self.fingerprints.clone(),
        )
    }

    /// Returns true if the library currently contains an asset at this library-relative path.
    pub fn contains(&self, path: &str) -> bool {
        self.assets.iter().any(|a| a.path == path)
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
        std::fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))?;
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

#[derive(Debug, Clone)]
pub struct LibraryAsset {
    pub name: String,
    /// Library-relative path with forward slashes (e.g., "terrain/stone/floor.png").
    /// This is the asset's identity at runtime and in map files.
    pub path: String,
    /// Parent folder relative path from library root (e.g., "terrain/stone" or "" for root)
    pub folder_path: String,
    /// File extension (e.g., "png", "jpg")
    pub extension: String,
    /// Full filesystem path for reading metadata
    pub full_path: PathBuf,
    /// Size and modification time when the library was scanned
    pub stamp: FileStamp,
}

impl LibraryAsset {
    /// The `lib://` asset path Bevy loads this asset from.
    pub fn asset_path(&self) -> bevy::asset::AssetPath<'static> {
        library_asset_path(&self.path)
    }
}

/// Library-relative path for a file inside the library, with forward slashes.
pub fn relative_to_library(library_path: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(library_path)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}

/// Scans assets from a library directory into the AssetLibrary resource
fn scan_library_at_path(library: &mut AssetLibrary, library_path: &Path) {
    library.assets.clear();
    library.error = None;
    library.generation += 1;

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
            // Skip the library's "maps" directory
            if current_path == base_path
                && path.file_name().and_then(|n| n.to_str()) == Some("maps")
            {
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
        // Normalize path separators to forward slashes for cross-platform consistency
        let folder_path = relative_to_library(base_path, current_path).unwrap_or_default();
        let Some(relative_path) = relative_to_library(base_path, &path) else {
            continue;
        };
        // Metadata only: reading contents would download online-only files from sync clients
        let stamp = std::fs::metadata(&path)
            .map(|m| FileStamp::of_metadata(&m))
            .unwrap_or_default();

        library.assets.push(LibraryAsset {
            name,
            path: relative_path,
            folder_path,
            extension,
            full_path: path,
            stamp,
        });
    }
}

/// Initial scan from default asset library location
pub fn scan_asset_library(mut library: ResMut<AssetLibrary>) {
    let library_path = library.library_path.clone();

    if library_path.exists() {
        library.metadata = load_library_metadata(&library_path);
    } else {
        // First run: create the default library (the only time startup writes to one)
        match create_library_directory(&library_path) {
            Ok(()) => {
                let mut metadata = new_library_metadata(&library_path);
                if let Err(e) = save_library_metadata(&library_path, &mut metadata) {
                    warn!("Failed to write default library manifest: {}", e);
                }
                library.metadata = metadata;
            }
            Err(e) => warn!("Failed to create default library: {}", e),
        }
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
    /// Tracks paths that failed to load (so we don't retry until the file changes)
    pub failed: HashMap<PathBuf, ()>,
    /// Size and modification time of the file each thumbnail (or failure) came from
    pub stamps: HashMap<PathBuf, FileStamp>,
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

    /// Forget one asset's thumbnail so it's regenerated (its file changed on disk).
    pub fn invalidate(&mut self, path: &Path, egui_textures: &mut EguiUserTextures) {
        if let Some(handle) = self.thumbnails.remove(path) {
            egui_textures.remove_image(&handle);
        }
        self.texture_ids.remove(path);
        self.failed.remove(path);
        self.stamps.remove(path);
    }

    /// Returns true if this asset's file changed since its thumbnail (or failure) was made.
    pub fn is_stale(&self, asset: &LibraryAsset) -> bool {
        self.stamps
            .get(&asset.full_path)
            .is_some_and(|stamp| *stamp != asset.stamp)
    }

    /// Clear the cache (call when switching libraries)
    pub fn clear(&mut self) {
        self.thumbnails.clear();
        self.texture_ids.clear();
        self.failed.clear();
        self.stamps.clear();
    }
}

/// Renders a thumbnail from an image file, at most THUMBNAIL_SIZE on its longer side.
/// For animated formats (GIF), only the first frame is used.
pub fn render_thumbnail(path: &Path) -> Option<image::RgbaImage> {
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

    Some(
        img.resize(thumb_w, thumb_h, FilterType::Triangle)
            .to_rgba8(),
    )
}

/// Wraps thumbnail pixels in a Bevy Image.
pub fn thumbnail_to_image(rgba: image::RgbaImage) -> Image {
    let (width, height) = rgba.dimensions();
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
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
            // Read only: opening a synced library must not write to it
            library.set_library_path(path.clone());
            library.metadata = load_library_metadata(&path);
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

    // A library that already has a manifest keeps its name and id
    let mut metadata = load_library_metadata(&path);
    if metadata.id.is_none() {
        metadata = new_library_metadata(&path);
        save_library_metadata(&path, &mut metadata)?;
    }
    library.metadata = metadata;

    library.set_library_path(path.clone());
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
