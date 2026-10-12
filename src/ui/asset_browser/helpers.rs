//! Helper functions for the asset browser.

use bevy_egui::egui;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::assets::{AssetLibrary, relative_to_library};

/// Scan the maps directory, including subfolders, and return a sorted list of
/// (display name, path). The display name is the path below maps/ without the extension,
/// e.g. "campaign/goblin cave". Hidden files and folders (sync client temp files) are skipped.
pub fn scan_maps_directory(maps_dir: &Path) -> Vec<(String, PathBuf)> {
    fn scan(maps_dir: &Path, dir: &Path, maps: &mut Vec<(String, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if path.is_dir() {
                scan(maps_dir, &path, maps);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && let Some(name) = relative_to_library(maps_dir, &path.with_extension(""))
            {
                maps.push((name, path));
            }
        }
    }

    let mut maps = Vec::new();
    scan(maps_dir, maps_dir, &mut maps);
    maps.sort_by_key(|a| a.0.to_lowercase());
    maps
}

/// If `name` (a map's display name) is a sync client's conflict copy, the display name of
/// the map it conflicts with. Recognizes Dropbox and Nextcloud ("cave (DESKTOP's conflicted
/// copy 2026-10-12)", "cave (conflicted copy 2026-10-12 140233)"), older Nextcloud/ownCloud
/// ("cave_conflict-20261012-140233") and Syncthing ("cave.sync-conflict-20261012-140233-ABC").
pub fn conflict_copy_original(name: &str) -> Option<&str> {
    if let Some(i) = name.find(".sync-conflict-") {
        return Some(&name[..i]);
    }
    if let Some(i) = name.rfind("_conflict-") {
        return Some(&name[..i]);
    }
    let open = name.rfind(" (")?;
    (name.ends_with(')') && name[open..].to_lowercase().contains("conflict")).then(|| &name[..open])
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
        "png" => egui::Color32::from_rgb(80, 140, 200), // Blue
        "jpg" | "jpeg" => egui::Color32::from_rgb(200, 140, 80), // Orange
        "webp" => egui::Color32::from_rgb(140, 200, 80), // Green
        "gif" => egui::Color32::from_rgb(200, 80, 140), // Pink
        "bmp" => egui::Color32::from_rgb(140, 80, 200), // Purple
        "tiff" | "tif" => egui::Color32::from_rgb(80, 200, 140), // Teal
        _ => egui::Color32::from_rgb(128, 128, 128),    // Gray
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_maps_directory_includes_subfolders_and_skips_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let maps = dir.path();
        std::fs::create_dir_all(maps.join("Campaign")).unwrap();
        std::fs::create_dir_all(maps.join(".stversions")).unwrap();
        for file in [
            "town.json",
            "Campaign/goblin cave.json",
            "notes.txt",
            ".rustforged-abc.tmp",
            ".stversions/town~20260101.json",
        ] {
            std::fs::write(maps.join(file), "{}").unwrap();
        }

        let names: Vec<String> = scan_maps_directory(maps)
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        assert_eq!(names, ["Campaign/goblin cave", "town"]);
    }

    #[test]
    fn test_conflict_copy_original_recognizes_sync_clients() {
        for (copy, original) in [
            ("cave (DESKTOP's conflicted copy 2026-10-12)", "cave"),
            ("cave (conflicted copy 2026-10-12 140233)", "cave"),
            ("Campaign/cave (Case Conflict)", "Campaign/cave"),
            ("cave_conflict-20261012-140233", "cave"),
            ("cave.sync-conflict-20261012-140233-ABCDEFG", "cave"),
        ] {
            assert_eq!(conflict_copy_original(copy), Some(original), "{copy}");
        }
        // Copies saved on purpose, and ordinary names, aren't conflicts
        for name in ["cave (LAPTOP copy)", "cave (2)", "cave", "goblin (boss)"] {
            assert_eq!(conflict_copy_original(name), None, "{name}");
        }
    }

    #[test]
    fn test_scan_maps_directory_missing_folder_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(scan_maps_directory(&dir.path().join("maps")).is_empty());
    }
}
