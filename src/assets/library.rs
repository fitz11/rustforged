use bevy::prelude::*;
use std::path::PathBuf;

use super::{AssetCategory, RefreshAssetLibrary};

#[derive(Resource, Default)]
pub struct AssetLibrary {
    pub library_path: PathBuf,
    pub assets: Vec<LibraryAsset>,
}

#[derive(Debug, Clone)]
pub struct LibraryAsset {
    pub name: String,
    pub path: PathBuf,
    pub relative_path: String,
    pub category: AssetCategory,
    pub handle: Option<Handle<Image>>,
}

pub fn scan_asset_library(mut library: ResMut<AssetLibrary>, asset_server: Res<AssetServer>) {
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

                let handle: Handle<Image> = asset_server.load(&relative_path);

                library.assets.push(LibraryAsset {
                    name,
                    path: path.clone(),
                    relative_path,
                    category: *category,
                    handle: Some(handle),
                });
            }
        }
    }

    info!("Loaded {} assets from library", library.assets.len());
}

fn is_image_file(path: &PathBuf) -> bool {
    let extensions = ["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif"];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn refresh_asset_library(
    mut events: MessageReader<RefreshAssetLibrary>,
    mut library: ResMut<AssetLibrary>,
    asset_server: Res<AssetServer>,
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

                    let handle: Handle<Image> = asset_server.load(&relative_path);

                    library.assets.push(LibraryAsset {
                        name,
                        path: path.clone(),
                        relative_path,
                        category: *category,
                        handle: Some(handle),
                    });
                }
            }
        }

        info!("Refreshed asset library: {} assets", library.assets.len());
    }
}
