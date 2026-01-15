//! Paste system for clipboard operations (Ctrl+V).

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map::{Layer, MapData, PlacedItem, Selected};

use super::super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
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

        commands.spawn((
            Sprite::from_image(texture),
            Transform {
                translation: new_pos.extend(z),
                rotation: Quat::from_rotation_z(clip_item.saved.rotation),
                scale: clip_item.saved.scale.extend(1.0),
            },
            PlacedItem {
                asset_path: clip_item.saved.asset_path.clone(),
                layer: clip_item.saved.layer,
                z_index: clip_item.saved.z_index,
            },
            render_layer,
            Selected, // Auto-select pasted item
        ));
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

        commands.spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, annotation_z)),
            DrawnPath {
                points: new_points,
                color: array_to_color(clip_path.saved.color),
                stroke_width: clip_path.saved.stroke_width,
            },
            AnnotationMarker,
            Selected,
        ));
    }

    // Paste lines
    for clip_line in &clipboard.lines {
        let line_center = (clip_line.saved.start + clip_line.saved.end) / 2.0;
        let translation = paste_pos + clip_line.offset - line_center;

        commands.spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, annotation_z)),
            DrawnLine {
                start: clip_line.saved.start + translation,
                end: clip_line.saved.end + translation,
                color: array_to_color(clip_line.saved.color),
                stroke_width: clip_line.saved.stroke_width,
            },
            AnnotationMarker,
            Selected,
        ));
    }

    // Paste text annotations
    for clip_text in &clipboard.texts {
        let new_pos = paste_pos + clip_text.offset;

        commands.spawn((
            Transform::from_translation(new_pos.extend(annotation_z)),
            TextAnnotation {
                content: clip_text.saved.content.clone(),
                font_size: clip_text.saved.font_size,
                color: array_to_color(clip_text.saved.color),
            },
            AnnotationMarker,
            Selected,
        ));
    }
}
