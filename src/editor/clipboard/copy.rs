//! Copy system for clipboard operations (Ctrl+C).

use bevy::prelude::*;

use super::super::params::SelectedObjects;
use super::types::Clipboard;

/// Copy selected items to clipboard (Ctrl+C)
pub fn handle_copy(mut clipboard: ResMut<Clipboard>, selected: SelectedObjects) {
    let objects: Vec<_> = selected.collect().into_iter().map(|(_, o)| o).collect();
    // Nothing selected leaves the clipboard as it was
    clipboard.copy(&objects);
}
