//! Paste system for clipboard operations (Ctrl+V).

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map::{Layer, MapData, PlacedItem, Selected};

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
    keyboard: Res<ButtonInput<KeyCode>>,
    clipboard: Res<Clipboard>,
    mut contexts: EguiContexts,
    asset_server: Res<AssetServer>,
    camera: CameraParams,
    selected_query: Query<Entity, With<Selected>>,
    map_data: Res<MapData>,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    // Check for Ctrl+V
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    if !ctrl || !keyboard.just_pressed(KeyCode::KeyV) {
        return;
    }

    // Don't paste if UI has keyboard focus
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

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
        let layer_locked = map_data
            .layers
            .iter()
            .find(|ld| ld.layer_type == clip_item.saved.layer)
            .map(|ld| ld.locked)
            .unwrap_or(false);

        if layer_locked {
            continue;
        }

        let new_pos = paste_pos + clip_item.offset;
        let z = clip_item.saved.layer.z_base() + clip_item.saved.z_index as f32;

        let texture: Handle<Image> = asset_server.load(&clip_item.saved.asset_path);

        // Items on non-player-visible layers go to render layer 1 (editor-only)
        let render_layer = if clip_item.saved.layer.is_player_visible() {
            RenderLayers::layer(0)
        } else {
            RenderLayers::layer(1)
        };

        let transform = Transform {
            translation: new_pos.extend(z),
            rotation: Quat::from_rotation_z(clip_item.saved.rotation),
            scale: clip_item.saved.scale.extend(1.0),
        };
        let entity = commands
            .spawn((
                Sprite::from_image(texture),
                transform,
                PlacedItem {
                    asset_path: clip_item.saved.asset_path.clone(),
                    layer: clip_item.saved.layer,
                    z_index: clip_item.saved.z_index,
                },
                render_layer,
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
    let annotation_locked = map_data
        .layers
        .iter()
        .find(|ld| ld.layer_type == Layer::Annotation)
        .map(|ld| ld.locked)
        .unwrap_or(false);

    if annotation_locked {
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
