//! Undo/Redo system for editor actions.
//!
//! This module provides a command history that allows users to undo and redo their actions.
//! All reversible editor operations (placement, deletion, movement, etc.) are recorded as
//! commands that can be undone and redone.
//!
//! ## Usage
//!
//! - **Ctrl+Z**: Undo the last action
//! - **Ctrl+Y** or **Ctrl+Shift+Z**: Redo the last undone action
//!
//! ## Supported Operations
//!
//! - Adding things to the map: placing, painting, pasting, drawing
//! - Removing them: deleting
//! - Changing them in place: moving, resizing, rotating
//!
//! Commands find things by their [`ItemId`](crate::map::ItemId), not by entity, so a step
//! still applies after an earlier undo respawned what it refers to.
//!
//! ## Module Structure
//!
//! - [`commands`] - EditorCommand enum defining all reversible operations
//! - [`data_types`] - MapObject, what the history remembers about a thing on the map
//! - [`command_history`] - CommandHistory resource for tracking state
//! - [`systems`] - Bevy systems for keyboard shortcuts
//! - [`execute`] - Carrying out undo and redo

mod command_history;
mod commands;
mod data_types;
mod execute;
mod recording;
mod systems;

#[cfg(test)]
mod tests;

// Re-exports
pub use command_history::CommandHistory;
pub use commands::EditorCommand;
pub use data_types::MapObject;
pub use recording::{RecordEditorCommand, record_commands};
pub use systems::{clear_history_on_map_change, handle_undo_redo};

/// Maximum number of commands to keep in history
pub(crate) const MAX_HISTORY_SIZE: usize = 100;
