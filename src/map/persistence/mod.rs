//! Map persistence system for saving and loading maps.
//!
//! Handles async file I/O for map data, including:
//! - Save/load with async task pooling
//! - Missing asset validation
//! - Multi-map management (OpenMaps)
//! - Dirty state tracking
//!
//! ## Module Structure
//!
//! - [`messages`] - Message types for map operations
//! - [`resources`] - Resource types for state tracking
//! - [`results`] - Result types for async operations
//! - [`helpers`] - Utility functions (color conversion, directory creation)
//! - [`save`] - Save system and task polling
//! - [`load`] - Load system and task polling
//! - [`map_state`] - New map and switch map systems
//! - [`dirty`] - Dirty state detection systems
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
mod helpers;
mod load;
mod map_state;
mod messages;
mod resources;
mod results;
mod save;

#[cfg(test)]
mod tests;

// Re-exports - Messages
pub use messages::{LoadMapRequest, NewMapRequest, SaveMapRequest, SwitchMapRequest};

// Re-exports - Resources
pub use resources::{
    AsyncMapOperation, CurrentMapFile, LoadValidationWarning, MapDirtyState, MapLoadError,
    MapSaveError, OpenMaps, SaveValidationWarning, UnsavedChangesDialog,
};

// Re-exports - Helpers
pub use helpers::ensure_maps_directory;

// Re-exports - Systems
pub use dirty::{detect_item_additions, detect_item_removals, detect_item_transforms};
pub use load::{load_map_system, poll_load_tasks};
pub use map_state::{new_map_system, switch_map_system};
pub use save::{poll_save_tasks, save_map_system};
