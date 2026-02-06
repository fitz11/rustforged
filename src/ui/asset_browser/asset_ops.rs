//! Asset file operations (rename, move).

use bevy::prelude::*;
use std::path::{Path, PathBuf};

/// Rename an asset file and update all map files that reference it.
pub fn rename_asset(
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

    // Calculate old library-relative path BEFORE rename
    // Maps now store library-relative paths (e.g., "terrain/stone/floor.png")
    let old_relative = {
        let folder = old_path
            .parent()
            .and_then(|p| p.strip_prefix(library_path).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let stem = old_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if folder.is_empty() {
            format!("{}.{}", stem, extension)
        } else {
            format!("{}/{}.{}", folder, stem, extension)
        }
    };

    // Rename the file
    std::fs::rename(old_path, &new_path)
        .map_err(|e| format!("Failed to rename file: {}", e))?;

    // Calculate new library-relative path AFTER rename
    let new_relative = {
        let folder = new_path
            .parent()
            .and_then(|p| p.strip_prefix(library_path).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if folder.is_empty() {
            new_filename.clone()
        } else {
            format!("{}/{}", folder, new_filename)
        }
    };

    // Update all map files in the library's maps folder
    let maps_dir = library_path.join("maps");
    if maps_dir.exists() {
        update_asset_paths_in_maps(&maps_dir, &old_relative, &new_relative)?;
    }

    Ok((new_path, old_relative, new_relative))
}

/// Update asset_path references in all map files.
pub fn update_asset_paths_in_maps(
    maps_dir: &Path,
    old_path: &str,
    new_path: &str,
) -> Result<(), String> {
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

/// Move an asset file to a different folder and update all map files that reference it.
pub fn move_asset(
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

    // Calculate old library-relative path BEFORE move
    // Maps now store library-relative paths (e.g., "terrain/stone/floor.png")
    let old_relative = {
        let folder = old_path
            .parent()
            .and_then(|p| p.strip_prefix(library_path).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if folder.is_empty() {
            filename.to_string()
        } else {
            format!("{}/{}", folder, filename)
        }
    };

    // Move the file
    std::fs::rename(old_path, &new_path)
        .map_err(|e| format!("Failed to move file: {}", e))?;

    // Calculate new library-relative path AFTER move
    let new_relative = if target_folder.is_empty() {
        filename.to_string()
    } else {
        format!("{}/{}", target_folder, filename)
    };

    // Update all map files in the library's maps folder
    let maps_dir = library_path.join("maps");
    if maps_dir.exists() {
        update_asset_paths_in_maps(&maps_dir, &old_relative, &new_relative)?;
    }

    Ok((new_path, old_relative, new_relative))
}
