//! Editor command enum for undo/redo operations.

use bevy::prelude::*;

use super::data_types::{LineData, PathData, PlacedItemData, TextData, TransformData};

/// A reversible command in the editor
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum EditorCommand {
    /// Item(s) were placed on the map
    PlaceItems { items: Vec<PlacedItemData> },
    /// Item(s) were deleted from the map
    DeleteItems { items: Vec<PlacedItemData> },
    /// Item(s) were moved/transformed
    MoveItems {
        /// Entity ID, old transform, new transform
        transforms: Vec<(Entity, TransformData, TransformData)>,
    },
    /// A freehand path was created
    CreatePath { entity: Entity, path: PathData },
    /// A freehand path was deleted
    DeletePath { path: PathData },
    /// A line was created
    CreateLine { entity: Entity, line: LineData },
    /// A line was deleted
    DeleteLine { line: LineData },
    /// A text annotation was created
    CreateText { entity: Entity, text: TextData },
    /// A text annotation was deleted
    DeleteText { text: TextData },
}
