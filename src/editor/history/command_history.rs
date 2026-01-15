//! Command history resource for tracking undo/redo state.

use bevy::prelude::*;

use super::commands::EditorCommand;
use super::MAX_HISTORY_SIZE;

/// Resource tracking command history for undo/redo
#[derive(Resource, Default)]
pub struct CommandHistory {
    /// Stack of commands that can be undone (most recent last)
    undo_stack: Vec<EditorCommand>,
    /// Stack of commands that can be redone (most recent last)
    redo_stack: Vec<EditorCommand>,
}

#[allow(dead_code)]
impl CommandHistory {
    /// Push a new command to the history
    pub fn push(&mut self, command: EditorCommand) {
        // Clear redo stack when a new action is performed
        self.redo_stack.clear();

        self.undo_stack.push(command);

        // Trim history if it exceeds max size
        while self.undo_stack.len() > MAX_HISTORY_SIZE {
            self.undo_stack.remove(0);
        }
    }

    /// Pop the last command for undo
    pub fn pop_undo(&mut self) -> Option<EditorCommand> {
        self.undo_stack.pop()
    }

    /// Pop the last command for redo
    pub fn pop_redo(&mut self) -> Option<EditorCommand> {
        self.redo_stack.pop()
    }

    /// Push a command to the redo stack (used after undo)
    pub fn push_redo(&mut self, command: EditorCommand) {
        self.redo_stack.push(command);
    }

    /// Push a command to the undo stack (used after redo)
    pub fn push_undo(&mut self, command: EditorCommand) {
        self.undo_stack.push(command);
    }

    /// Check if there are commands to undo
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if there are commands to redo
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the count of undoable commands
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the count of redoable commands
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
