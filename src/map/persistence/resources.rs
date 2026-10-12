//! Resource types for map persistence state tracking.

use bevy::prelude::*;
use bevy::tasks::Task;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::assets::AssetFingerprint;

use super::results::{LoadResult, SaveResult};

#[derive(Resource, Default)]
pub struct MapLoadError {
    pub message: Option<String>,
}

/// Resource tracking save operation errors for display to user.
#[derive(Resource, Default)]
pub struct MapSaveError {
    pub message: Option<String>,
}

/// What happened to a map's asset references when it was opened, shown to the user when
/// anything had to be relinked or is missing.
#[derive(Resource, Default)]
pub struct MapAssetReport {
    /// Whether to show the report dialog
    pub show: bool,
    /// Name of the map the report is about
    pub map_name: String,
    /// References whose file was found under a different path: (old path, new path)
    pub relinked: Vec<(String, String)>,
    /// References with no matching file in the library; shown as placeholders
    pub missing: Vec<String>,
}

/// Why a save stopped before writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveConflictKind {
    /// The file changed on disk since this map was opened or last saved here
    Changed,
    /// A new map (or a copy) would overwrite a file that already exists
    Exists,
    /// The file this map was opened from is gone (moved, renamed or deleted elsewhere)
    Deleted,
}

/// A save that was stopped because the file on disk isn't the version this map came from.
#[derive(Debug, Clone)]
pub struct SaveConflict {
    pub path: PathBuf,
    pub kind: SaveConflictKind,
    /// Machine that saved the version on disk, if the file says
    pub saved_by: Option<String>,
    /// When the version on disk was written
    pub modified: Option<SystemTime>,
}

/// Resource holding a save conflict for the user to resolve.
#[derive(Resource, Default)]
pub struct SaveConflictDialog {
    pub conflict: Option<SaveConflict>,
}

/// Resource tracking async map I/O operations for modal dialog
#[derive(Resource, Default)]
pub struct AsyncMapOperation {
    /// Whether a save operation is in progress
    pub is_saving: bool,
    /// Whether a load operation is in progress
    pub is_loading: bool,
    /// Description of the current operation
    pub operation_description: Option<String>,
}

impl AsyncMapOperation {
    pub fn is_busy(&self) -> bool {
        self.is_saving || self.is_loading
    }
}

/// Component for save task
#[derive(Component)]
pub struct SaveMapTask(pub Task<SaveResult>);

/// Component for load task
#[derive(Component)]
pub struct LoadMapTask(pub Task<LoadResult>);

/// Resource tracking the currently loaded map file path
#[derive(Resource, Default)]
pub struct CurrentMapFile {
    pub path: Option<PathBuf>,
}

/// Resource tracking if the current map has unsaved changes
#[derive(Resource, Default)]
pub struct MapDirtyState {
    pub is_dirty: bool,
    /// Count of entities when map was last saved/loaded (for change detection)
    pub last_known_item_count: usize,
    pub last_known_annotation_count: usize,
    /// Number of frames for which change detection should be ignored. Set after
    /// a load/new/switch despawns+respawns entities so the resulting Added/
    /// Removed/Changed wave doesn't immediately re-mark a clean map as dirty.
    pub suppress_detection: u32,
}

impl MapDirtyState {
    /// Suppress change detection for the next couple of frames. Used after
    /// programmatic entity churn (load/new/switch) so the spawn/despawn wave
    /// isn't mistaken for user edits. Two frames covers either ordering of the
    /// (unordered) detection systems relative to the spawning system.
    pub fn suppress_change_detection(&mut self) {
        self.suppress_detection = 2;
    }
}

/// Represents a map that's open in memory
#[derive(Clone)]
pub struct OpenMap {
    pub id: u64,
    pub name: String,
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    pub saved_state: Option<crate::map::SavedMap>,
    /// The map file's stable id, once it has been saved or loaded
    pub map_id: Option<String>,
    /// Content hash of the file as this app last read or wrote it. A save compares it with
    /// the file on disk to notice changes made elsewhere (e.g. synced from another machine).
    pub disk_hash: Option<String>,
    /// Asset fingerprints carried from the map file. Kept so references to files that are
    /// missing right now (e.g. still syncing) are saved with their fingerprint intact.
    pub asset_fingerprints: BTreeMap<String, AssetFingerprint>,
}

/// Resource tracking all open maps
#[derive(Resource)]
pub struct OpenMaps {
    pub maps: HashMap<u64, OpenMap>,
    pub active_map_id: Option<u64>,
    pub next_id: u64,
}

impl Default for OpenMaps {
    fn default() -> Self {
        // Start with one untitled map
        let mut maps = HashMap::new();
        maps.insert(
            0,
            OpenMap {
                id: 0,
                name: "Untitled Map".to_string(),
                path: None,
                is_dirty: false,
                saved_state: None,
                map_id: None,
                disk_hash: None,
                asset_fingerprints: BTreeMap::new(),
            },
        );
        Self {
            maps,
            active_map_id: Some(0),
            next_id: 1,
        }
    }
}

impl OpenMaps {
    /// Get the currently active map
    pub fn active_map(&self) -> Option<&OpenMap> {
        self.active_map_id.and_then(|id| self.maps.get(&id))
    }

    /// Get the currently active map mutably
    pub fn active_map_mut(&mut self) -> Option<&mut OpenMap> {
        self.active_map_id.and_then(|id| self.maps.get_mut(&id))
    }

    /// Check if any open map has unsaved changes
    #[allow(dead_code)]
    pub fn has_any_unsaved(&self) -> bool {
        self.maps.values().any(|m| m.is_dirty)
    }

    /// Get list of maps with unsaved changes
    #[allow(dead_code)]
    pub fn unsaved_maps(&self) -> Vec<&OpenMap> {
        self.maps.values().filter(|m| m.is_dirty).collect()
    }
}

/// UI state for unsaved changes confirmation dialogs
#[derive(Resource, Default)]
pub struct UnsavedChangesDialog {
    /// Show dialog for switching maps
    #[allow(dead_code)]
    pub show_switch_confirmation: bool,
    /// The map ID we want to switch to
    #[allow(dead_code)]
    pub pending_switch_id: Option<u64>,
    /// Show dialog for closing app
    pub show_close_confirmation: bool,
    /// Show dialog for loading a new map
    #[allow(dead_code)]
    pub show_load_confirmation: bool,
    /// Path to load after confirmation
    #[allow(dead_code)]
    pub pending_load_path: Option<PathBuf>,
}
