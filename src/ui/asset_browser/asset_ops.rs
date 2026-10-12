//! Asset file operations (rename, move).
//!
//! Only the file itself changes. Open maps are updated in memory (RetargetAssetRequest);
//! map files on disk are left alone and find the asset again by fingerprint when opened.
//! Rewriting every map here would fan one rename out into many writes, which on a synced
//! library turns into conflict copies.

use std::path::{Path, PathBuf};

use crate::assets::relative_to_library;

/// Rename an asset file. Returns the new file path and the old and new library-relative paths.
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

    let extension = old_path.extension().and_then(|e| e.to_str()).unwrap_or("");

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

    let old_relative =
        relative_to_library(library_path, old_path).ok_or("Asset is outside the library")?;

    std::fs::rename(old_path, &new_path).map_err(|e| format!("Failed to rename file: {}", e))?;

    let new_relative =
        relative_to_library(library_path, &new_path).ok_or("Asset is outside the library")?;
    Ok((new_path, old_relative, new_relative))
}

/// Move an asset file to a different folder. Returns the new file path and the old and new
/// library-relative paths.
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

    let old_relative =
        relative_to_library(library_path, old_path).ok_or("Asset is outside the library")?;

    std::fs::rename(old_path, &new_path).map_err(|e| format!("Failed to move file: {}", e))?;

    let new_relative =
        relative_to_library(library_path, &new_path).ok_or("Asset is outside the library")?;
    Ok((new_path, old_relative, new_relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rename_leaves_map_files_alone() {
        let library = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(library.path().join("tokens")).unwrap();
        std::fs::create_dir_all(library.path().join("maps")).unwrap();
        std::fs::write(library.path().join("orc.png"), "orc").unwrap();
        let map =
            r#"{"placed_items":[{"asset_path":"tokens/orc.png"},{"asset_path":"big_orc.png"}]}"#;
        std::fs::write(library.path().join("maps/cave.json"), map).unwrap();

        let (new_path, old_relative, new_relative) =
            rename_asset(&library.path().join("orc.png"), "goblin", library.path()).unwrap();

        assert_eq!(new_path, library.path().join("goblin.png"));
        assert_eq!(old_relative, "orc.png");
        assert_eq!(new_relative, "goblin.png");
        // A root-level rename used to rewrite "tokens/orc.png" and "big_orc.png" too
        let after = std::fs::read_to_string(library.path().join("maps/cave.json")).unwrap();
        assert_eq!(after, map);
    }

    #[test]
    fn test_move_returns_library_relative_paths() {
        let library = tempfile::tempdir().unwrap();
        std::fs::write(library.path().join("orc.png"), "orc").unwrap();

        let (new_path, old_relative, new_relative) = move_asset(
            &library.path().join("orc.png"),
            "tokens/monsters",
            library.path(),
        )
        .unwrap();

        assert_eq!(new_path, library.path().join("tokens/monsters/orc.png"));
        assert_eq!(old_relative, "orc.png");
        assert_eq!(new_relative, "tokens/monsters/orc.png");
        assert!(new_path.exists());
    }
}
