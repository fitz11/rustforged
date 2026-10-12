//! Clipboard data types for copy/paste operations.

use bevy::prelude::*;

use crate::map::{SavedLine, SavedPath, SavedPlacedItem, SavedTextBox};

/// Clipboard data for a placed item with offset from selection centroid
#[derive(Clone)]
pub struct ClipboardPlacedItem {
    pub saved: SavedPlacedItem,
    pub offset: Vec2,
}

/// Clipboard data for a path annotation with offset from selection centroid
#[derive(Clone)]
pub struct ClipboardPath {
    pub saved: SavedPath,
    pub offset: Vec2,
}

/// Clipboard data for a line annotation with offset from selection centroid
#[derive(Clone)]
pub struct ClipboardLine {
    pub saved: SavedLine,
    pub offset: Vec2,
}

/// Clipboard data for a text annotation with offset from selection centroid
#[derive(Clone)]
pub struct ClipboardText {
    pub saved: SavedTextBox,
    pub offset: Vec2,
}

/// Resource that holds copied items
#[derive(Resource, Default)]
pub struct Clipboard {
    pub placed_items: Vec<ClipboardPlacedItem>,
    pub paths: Vec<ClipboardPath>,
    pub lines: Vec<ClipboardLine>,
    pub texts: Vec<ClipboardText>,
}

impl Clipboard {
    /// Point copied placed items at a different asset path (after a rename, move or relink).
    pub fn retarget_asset(&mut self, old_path: &str, new_path: &str) {
        for item in self
            .placed_items
            .iter_mut()
            .filter(|i| i.saved.asset_path == old_path)
        {
            item.saved.asset_path = new_path.to_string();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.placed_items.is_empty()
            && self.paths.is_empty()
            && self.lines.is_empty()
            && self.texts.is_empty()
    }

    pub fn clear(&mut self) {
        self.placed_items.clear();
        self.paths.clear();
        self.lines.clear();
        self.texts.clear();
    }
}
