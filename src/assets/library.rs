use bevy::prelude::*;
use std::path::{Path, PathBuf};

use super::{AssetCategory, RefreshAssetLibrary};

#[derive(Resource, Default)]
pub struct AssetLibrary {
    pub library_path: PathBuf,
    pub assets: Vec<LibraryAsset>,
}

#[derive(Debug, Clone)]
pub struct LibraryAsset {
    pub name: String,
    pub relative_path: String,
    pub category: AssetCategory,
}

/// Currently scans from hard coded asset libary location. Eventually plan to add support for
/// setting custom asset folder locations.
pub fn scan_asset_library(mut library: ResMut<AssetLibrary>) {
    let library_path = PathBuf::from("assets/library");
    library.library_path = library_path.clone();

    for category in AssetCategory::all() {
        let category_path = library_path.join(category.folder_name());

        if !category_path.exists() {
            if let Err(e) = std::fs::create_dir_all(&category_path) {
                warn!("Failed to create directory {:?}: {}", category_path, e);
            }
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

                let relative_path = format!(
                    "library/{}/{}",
                    category.folder_name(),
                    path.file_name().unwrap().to_str().unwrap()
                );

                library.assets.push(LibraryAsset {
                    name,
                    relative_path,
                    category: *category,
                });
            }
        }
    }

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
        // Clear existing assets
        library.assets.clear();

        let library_path = PathBuf::from("assets/library");
        library.library_path = library_path.clone();

        for category in AssetCategory::all() {
            let category_path = library_path.join(category.folder_name());

            if !category_path.exists() {
                if let Err(e) = std::fs::create_dir_all(&category_path) {
                    warn!("Failed to create directory {:?}: {}", category_path, e);
                }
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

                    let relative_path = format!(
                        "library/{}/{}",
                        category.folder_name(),
                        path.file_name().unwrap().to_str().unwrap()
                    );

                    library.assets.push(LibraryAsset {
                        name,
                        relative_path,
                        category: *category,
                    });
                }
            }
        }

        info!("Refreshed asset library: {} assets", library.assets.len());
    }
}
