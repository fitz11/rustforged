//! The library manifest, `rustforged-library.json`.
//!
//! Holds the library's name and a stable id. It's a visible file because sync clients
//! don't reliably sync dotfiles. It's only written when a library is created or renamed:
//! opening a library never writes to it, so machines sharing a synced library don't each
//! create their own copy.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Filename of the library manifest, at the library root.
pub const LIBRARY_MANIFEST_FILE: &str = "rustforged-library.json";

/// Hidden manifest written by earlier versions; still read so libraries keep their name.
const LEGACY_MANIFEST_FILE: &str = ".library.json";

/// Manifest format this version writes.
const LIBRARY_FORMAT_VERSION: u32 = 1;

/// Metadata for an asset library, stored in its manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryMetadata {
    #[serde(default)]
    pub format_version: u32,
    /// Stable identifier, the same on every machine the library syncs to. `None` for a
    /// folder that was opened but never created or renamed in the app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// User-defined name for the library
    pub name: String,
}

impl Default for LibraryMetadata {
    fn default() -> Self {
        Self {
            format_version: LIBRARY_FORMAT_VERSION,
            id: None,
            name: "Unnamed Library".to_string(),
        }
    }
}

/// Name for a library without a manifest: its folder name.
fn folder_name(library_path: &Path) -> String {
    library_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unnamed Library")
        .to_string()
}

fn read_manifest(path: &Path) -> Option<LibraryMetadata> {
    let json = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&json) {
        Ok(metadata) => Some(metadata),
        Err(e) => {
            warn!("Failed to parse library manifest {:?}: {}", path, e);
            None
        }
    }
}

/// Reads a library's manifest without writing anything. Falls back to the legacy hidden
/// manifest's name, then to the folder name.
pub fn load_library_metadata(library_path: &Path) -> LibraryMetadata {
    if let Some(metadata) = read_manifest(&library_path.join(LIBRARY_MANIFEST_FILE)) {
        if metadata.format_version > LIBRARY_FORMAT_VERSION {
            warn!(
                "Library manifest format {} is newer than this version supports ({})",
                metadata.format_version, LIBRARY_FORMAT_VERSION
            );
        }
        return metadata;
    }

    let name = read_manifest(&library_path.join(LEGACY_MANIFEST_FILE))
        .map(|legacy| legacy.name)
        .unwrap_or_else(|| folder_name(library_path));
    LibraryMetadata {
        name,
        ..LibraryMetadata::default()
    }
}

/// Metadata for a library being created in `library_path`, named after the folder.
pub fn new_library_metadata(library_path: &Path) -> LibraryMetadata {
    LibraryMetadata {
        name: folder_name(library_path),
        ..LibraryMetadata::default()
    }
}

/// Writes the library manifest, giving the library an id if it doesn't have one yet.
pub fn save_library_metadata(
    library_path: &Path,
    metadata: &mut LibraryMetadata,
) -> Result<(), String> {
    metadata.format_version = LIBRARY_FORMAT_VERSION;
    metadata
        .id
        .get_or_insert_with(|| uuid::Uuid::new_v4().to_string());

    let json = serde_json::to_string_pretty(metadata)
        .map_err(|e| format!("Failed to serialize library manifest: {}", e))?;
    let manifest_path = library_path.join(LIBRARY_MANIFEST_FILE);
    std::fs::write(&manifest_path, json)
        .map_err(|e| format!("Failed to write library manifest: {}", e))?;

    info!("Saved library manifest to {:?}", manifest_path);
    Ok(())
}

/// Returns true for an archive entry name that is a library manifest at the archive root.
pub fn is_root_manifest(entry_name: &str) -> bool {
    entry_name == LIBRARY_MANIFEST_FILE || entry_name == LEGACY_MANIFEST_FILE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_without_manifest_uses_folder_name_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let library = dir.path().join("Dungeon Tiles");
        std::fs::create_dir(&library).unwrap();

        let metadata = load_library_metadata(&library);

        assert_eq!(metadata.name, "Dungeon Tiles");
        assert!(metadata.id.is_none());
        assert_eq!(std::fs::read_dir(&library).unwrap().count(), 0);
    }

    #[test]
    fn test_load_reads_legacy_hidden_manifest_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(LEGACY_MANIFEST_FILE),
            r#"{"name": "Old Library"}"#,
        )
        .unwrap();

        let metadata = load_library_metadata(dir.path());

        assert_eq!(metadata.name, "Old Library");
        assert!(metadata.id.is_none());
    }

    #[test]
    fn test_save_assigns_id_once_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let mut metadata = new_library_metadata(dir.path());
        save_library_metadata(dir.path(), &mut metadata).unwrap();
        let id = metadata.id.clone().expect("save assigns an id");

        metadata.name = "Renamed".to_string();
        save_library_metadata(dir.path(), &mut metadata).unwrap();
        let loaded = load_library_metadata(dir.path());

        assert_eq!(loaded.id, Some(id));
        assert_eq!(loaded.name, "Renamed");
        assert_eq!(loaded.format_version, LIBRARY_FORMAT_VERSION);
        assert!(dir.path().join(LIBRARY_MANIFEST_FILE).exists());
    }

    #[test]
    fn test_manifest_takes_precedence_over_legacy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(LEGACY_MANIFEST_FILE), r#"{"name": "Old"}"#).unwrap();
        std::fs::write(
            dir.path().join(LIBRARY_MANIFEST_FILE),
            r#"{"format_version": 1, "id": "abc", "name": "New"}"#,
        )
        .unwrap();

        let metadata = load_library_metadata(dir.path());

        assert_eq!(metadata.name, "New");
        assert_eq!(metadata.id.as_deref(), Some("abc"));
    }
}
