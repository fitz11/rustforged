//! Paste system for clipboard operations (Ctrl+V).

use bevy::prelude::*;

use crate::map::{Layer, MapData, Selected};

use super::super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use super::super::history::{
    EditorCommand, LineData, PathData, PlacedItemData, RecordEditorCommand, TextData, TransformData,
};
use super::super::params::CameraParams;
use super::helpers::{array_to_color, saved_path_center};
use super::types::Clipboard;

/// Paste clipboard items at cursor position (Ctrl+V)
#[allow(clippy::too_many_arguments)]
pub fn handle_paste(
    mut commands: Commands,
    clipboard: Res<Clipboard>,
    asset_server: Res<AssetServer>,
    camera: CameraParams,
    selected_query: Query<Entity, With<Selected>>,
    map_data: Res<MapData>,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    // Check clipboard has content
    if clipboard.is_empty() {
        return;
    }

    // Get cursor world position
    let Some(paste_pos) = camera.cursor_world_pos() else {
        return;
    };

    // Clear current selection
    for entity in selected_query.iter() {
        commands.entity(entity).remove::<Selected>();
    }

    // Accumulate pasted placed items so the batch is one undo step.
    let mut pasted_items: Vec<PlacedItemData> = Vec::new();

    // Paste placed items
    for clip_item in &clipboard.placed_items {
        // Check if target layer is locked
        if map_data.is_layer_locked(clip_item.saved.layer) {
            continue;
        }

        let item = clip_item.saved.placed_item();
        let transform = Transform {
            translation: (paste_pos + clip_item.offset).extend(item.depth()),
            ..clip_item.saved.transform()
        };
        let entity = commands
            .spawn((
                item.bundle(&asset_server, transform),
                Selected, // Auto-select pasted item
            ))
            .id();

        pasted_items.push(PlacedItemData {
            entity,
            asset_path: clip_item.saved.asset_path.clone(),
            layer: clip_item.saved.layer,
            z_index: clip_item.saved.z_index,
            transform: TransformData::from(&transform),
        });
    }

    if !pasted_items.is_empty() {
        history_writer.write(RecordEditorCommand {
            command: EditorCommand::PlaceItems {
                items: pasted_items,
            },
        });
    }

    // Check if annotation layer is locked
    if map_data.is_layer_locked(Layer::Annotation) {
        return;
    }

    let annotation_z = Layer::Annotation.z_base();

    // Paste paths
    for clip_path in &clipboard.paths {
        // Translate all points to new position
        let center = saved_path_center(&clip_path.saved);
        let translation = paste_pos + clip_path.offset - center;

        let new_points: Vec<Vec2> = clip_path
            .saved
            .points
            .iter()
            .map(|p| *p + translation)
            .collect();

        let color = array_to_color(clip_path.saved.color);
        let stroke_width = clip_path.saved.stroke_width;
        let entity = commands
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, annotation_z)),
                DrawnPath {
                    points: new_points.clone(),
                    color,
                    stroke_width,
                },
                AnnotationMarker,
                Selected,
            ))
            .id();

        history_writer.write(RecordEditorCommand {
            command: EditorCommand::CreatePath {
                entity,
                path: PathData {
                    points: new_points,
                    color,
                    stroke_width,
                },
            },
        });
    }

    // Paste lines
    for clip_line in &clipboard.lines {
        let line_center = (clip_line.saved.start + clip_line.saved.end) / 2.0;
        let translation = paste_pos + clip_line.offset - line_center;

        let start = clip_line.saved.start + translation;
        let end = clip_line.saved.end + translation;
        let color = array_to_color(clip_line.saved.color);
        let stroke_width = clip_line.saved.stroke_width;
        let entity = commands
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, annotation_z)),
                DrawnLine {
                    start,
                    end,
                    color,
                    stroke_width,
                },
                AnnotationMarker,
                Selected,
            ))
            .id();

        history_writer.write(RecordEditorCommand {
            command: EditorCommand::CreateLine {
                entity,
                line: LineData {
                    start,
                    end,
                    color,
                    stroke_width,
                },
            },
        });
    }

    // Paste text annotations
    for clip_text in &clipboard.texts {
        let new_pos = paste_pos + clip_text.offset;

        let content = clip_text.saved.content.clone();
        let font_size = clip_text.saved.font_size;
        let color = array_to_color(clip_text.saved.color);
        let entity = commands
            .spawn((
                Transform::from_translation(new_pos.extend(annotation_z)),
                TextAnnotation {
                    content: content.clone(),
                    font_size,
                    color,
                },
                AnnotationMarker,
                Selected,
            ))
            .id();

        history_writer.write(RecordEditorCommand {
            command: EditorCommand::CreateText {
                entity,
                text: TextData {
                    text: content,
                    position: new_pos,
                    color,
                    font_size,
                },
            },
        });
    }
}
