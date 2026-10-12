//! Clipboard data types for copy/paste operations.

use bevy::prelude::*;

use crate::map::{SavedLine, SavedPath, SavedPlacedItem, SavedTextBox};

use super::super::history::MapObject;
use super::helpers::{color_to_array, path_center};

/// Where an object sits, for keeping copied objects in place relative to each other
fn anchor(object: &MapObject) -> Vec2 {
    match object {
        MapObject::Item { transform, .. } => transform.translation.truncate(),
        MapObject::Path { path, .. } => path_center(path),
        MapObject::Line { line, .. } => (line.start + line.end) / 2.0,
        MapObject::Text { position, .. } => *position,
    }
}

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
    /// Replaces the clipboard with `objects`, each remembered by its offset from their
    /// common center so a paste keeps them in place relative to each other.
    pub fn copy(&mut self, objects: &[MapObject]) {
        if objects.is_empty() {
            return;
        }
        self.clear();
        let centroid = objects.iter().map(anchor).sum::<Vec2>() / objects.len() as f32;

        for object in objects {
            let offset = anchor(object) - centroid;
            match object {
                MapObject::Item {
                    item, transform, ..
                } => self.placed_items.push(ClipboardPlacedItem {
                    saved: SavedPlacedItem::from_entity(item, transform),
                    offset,
                }),
                MapObject::Path { path, .. } => self.paths.push(ClipboardPath {
                    saved: SavedPath {
                        points: path.points.clone(),
                        color: color_to_array(path.color),
                        stroke_width: path.stroke_width,
                    },
                    offset,
                }),
                MapObject::Line { line, .. } => self.lines.push(ClipboardLine {
                    saved: SavedLine {
                        start: line.start,
                        end: line.end,
                        color: color_to_array(line.color),
                        stroke_width: line.stroke_width,
                    },
                    offset,
                }),
                MapObject::Text { text, position, .. } => self.texts.push(ClipboardText {
                    saved: SavedTextBox {
                        position: *position,
                        content: text.content.clone(),
                        font_size: text.font_size,
                        color: color_to_array(text.color),
                    },
                    offset,
                }),
            }
        }
    }

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
