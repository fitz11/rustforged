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
//! - Item placement and deletion
//! - Item movement (transform changes)
//! - Annotation creation and deletion (paths, lines, text)
//!
//! ## Module Structure
//!
//! - [`commands`] - EditorCommand enum defining all reversible operations
//! - [`data_types`] - Serializable data types for undo/redo state
//! - [`command_history`] - CommandHistory resource for tracking state
//! - [`systems`] - Bevy systems for keyboard shortcuts
//! - [`execute`] - Execute functions for undo/redo operations
//! - [`spawn_helpers`] - Helper functions for spawning entities

mod command_history;
mod commands;
mod data_types;
mod execute;
mod spawn_helpers;
mod systems;

#[cfg(test)]
mod tests;

// Re-exports
pub use command_history::CommandHistory;
pub use systems::{handle_redo, handle_undo};

/// Maximum number of commands to keep in history
#[allow(dead_code)]
pub(crate) const MAX_HISTORY_SIZE: usize = 100;
