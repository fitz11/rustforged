//! Helper functions for spawning entities during undo/redo.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::map::{Layer, PlacedItem};

use super::super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use super::data_types::{LineData, PathData, PlacedItemData, TextData};

/// Helper to spawn a placed item from PlacedItemData
pub fn spawn_placed_item(
    commands: &mut Commands,
    asset_server: &AssetServer,
    data: &PlacedItemData,
) -> Entity {
    let texture_handle: Handle<Image> = asset_server.load(&data.asset_path);

    commands
        .spawn((
            Sprite::from_image(texture_handle),
            Transform::from(data.transform),
            PlacedItem {
                asset_path: data.asset_path.clone(),
                layer: data.layer,
                z_index: data.z_index,
            },
            RenderLayers::from_layers(&[0, 1]),
        ))
        .id()
}

/// Helper to spawn a drawn path from PathData
pub fn spawn_path(commands: &mut Commands, data: &PathData) -> Entity {
    commands
        .spawn((
            DrawnPath {
                points: data.points.clone(),
                color: data.color,
                stroke_width: data.stroke_width,
            },
            Transform::from_xyz(0.0, 0.0, Layer::Annotation.z_base()),
            AnnotationMarker,
        ))
        .id()
}

/// Helper to spawn a drawn line from LineData
pub fn spawn_line(commands: &mut Commands, data: &LineData) -> Entity {
    commands
        .spawn((
            DrawnLine {
                start: data.start,
                end: data.end,
                color: data.color,
                stroke_width: data.stroke_width,
            },
            Transform::from_xyz(0.0, 0.0, Layer::Annotation.z_base()),
            AnnotationMarker,
        ))
        .id()
}

/// Helper to spawn a text annotation from TextData
pub fn spawn_text(commands: &mut Commands, data: &TextData) -> Entity {
    commands
        .spawn((
            TextAnnotation {
                content: data.text.clone(),
                color: data.color,
                font_size: data.font_size,
            },
            Transform::from_xyz(data.position.x, data.position.y, Layer::Annotation.z_base()),
            AnnotationMarker,
        ))
        .id()
}
