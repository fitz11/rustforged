//! Map persistence system for saving and loading maps.
//!
//! Handles async file I/O for map data, including:
//! - Save/load with async task pooling
//! - Resolving asset references that moved, and reporting missing ones
//! - Multi-map management (OpenMaps)
//! - Dirty state tracking
//!
//! ## Module Structure
//!
//! - [`messages`] - Message types for map operations
//! - [`resources`] - Resource types for state tracking
//! - [`results`] - Result types for async operations
//! - [`helpers`] - Utility functions (color conversion)
//! - [`save`] - Save system and task polling
//! - [`load`] - Load system and task polling
//! - [`map_state`] - New map and switch map systems
//! - [`dirty`] - Dirty state detection systems
//! - [`disk_changes`] - Noticing the open map's file change on disk
//! - [`recovery`] - Autosave to a recovery folder, offered back after a crash
//!
//! ## Key Types
//!
//! - [`OpenMaps`] - Tracks all open maps in memory
//! - [`MapDirtyState`] - Tracks unsaved changes
//! - [`AsyncMapOperation`] - Tracks async I/O state
//!
//! ## Systems
//!
//! - [`save_map_system`] - Starts async save operation
//! - [`poll_save_tasks`] - Polls save task completion
//! - [`load_map_system`] - Starts async load operation
//! - [`poll_load_tasks`] - Polls load task completion
//! - [`new_map_system`] - Creates a new blank map
//! - [`switch_map_system`] - Switches between open maps

mod dirty;
mod disk_changes;
mod helpers;
mod load;
mod map_state;
mod messages;
mod recovery;
mod resources;
mod results;
mod save;

#[cfg(test)]
mod tests;

// Re-exports - Messages
pub use messages::{
    LoadMapRequest, MapReplaced, MapSaveFinished, NewMapRequest, SaveMapRequest, SaveOutcome,
    SwitchMapRequest,
};

// Re-exports - Resources
pub use resources::{
    AsyncMapOperation, CurrentMapFile, MapAssetReport, MapChangedOnDisk, MapDirtyState,
    MapLoadError, MapSaveError, OpenMaps, SaveConflict, SaveConflictDialog, SaveConflictKind,
};

// Re-exports - Systems
pub use dirty::{
    decay_dirty_suppression, detect_item_additions, detect_item_removals, detect_item_transforms,
};
pub use disk_changes::detect_open_map_changes;
pub use load::{load_map_system, poll_load_tasks};
pub use map_state::{new_map_system, remember_player_view, switch_map_system};
pub use recovery::{
    Autosave, RecoverMapRequest, RecoveryOffer, autosave_recovery_file, find_recovery_files,
    recover_map_system, remove_recovery_files_on_exit, remove_unneeded_recovery_files,
};
pub use save::{poll_save_tasks, save_map_system};
