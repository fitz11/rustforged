//! Resource types for map persistence state tracking.

use bevy::prelude::*;
use bevy::tasks::Task;
use std::collections::HashMap;
use std::path::PathBuf;

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

/// Resource for pre-save validation warnings about missing assets.
#[derive(Resource, Default)]
pub struct SaveValidationWarning {
    /// Whether to show the warning dialog
    pub show: bool,
    /// List of asset paths that are missing
    pub missing_assets: Vec<String>,
    /// The path we want to save to after user confirmation
    pub pending_save_path: Option<PathBuf>,
}

/// Resource for load-time validation warnings about missing assets.
#[derive(Resource, Default)]
pub struct LoadValidationWarning {
    /// Whether to show the warning dialog
    pub show: bool,
    /// List of asset paths that are missing from the library
    pub missing_assets: Vec<String>,
    /// The map file that failed to load
    pub map_path: Option<PathBuf>,
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
}

/// Represents a map that's open in memory
#[derive(Clone)]
pub struct OpenMap {
    pub id: u64,
    pub name: String,
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    pub saved_state: Option<crate::map::SavedMap>,
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
    #[allow(dead_code)]
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
