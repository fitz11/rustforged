//! Helper functions for the asset browser.

use bevy_egui::egui;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::assets::AssetLibrary;

/// Scan the maps directory and return sorted list of map names (without extension).
pub fn scan_maps_directory(maps_dir: &std::path::Path) -> Vec<(String, PathBuf)> {
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

/// Discover all folders in the library from asset paths.
pub fn discover_folders(library: &AssetLibrary) -> Vec<String> {
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

/// Sanitize a map name for use as a filename.
pub fn sanitize_map_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Get a color for the preview square based on file extension.
pub fn extension_color(ext: &str) -> egui::Color32 {
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
