//! Cut system for clipboard operations (Ctrl+X).

use bevy::prelude::*;

use super::super::history::{RecordEditorCommand, remove_from_map};
use super::super::params::SelectedObjects;
use super::types::Clipboard;

/// Cut selected items to clipboard (Ctrl+X): copy, then delete as one undo step
pub fn handle_cut(
    mut commands: Commands,
    mut clipboard: ResMut<Clipboard>,
    selected: SelectedObjects,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    let selection = selected.collect();
    let objects: Vec<_> = selection.iter().map(|(_, o)| o.clone()).collect();
    clipboard.copy(&objects);
    remove_from_map(&mut commands, &mut history_writer, selection);
}
