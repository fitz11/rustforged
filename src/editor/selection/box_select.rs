//! Box selection handling.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::editor::annotations::{
    is_annotation_layer_locked, is_annotation_layer_visible, line_overlaps_rect, path_overlaps_rect,
    text_overlaps_rect,
};
use crate::editor::params::{is_cursor_over_ui, AnnotationQueries, CameraParams};
use crate::editor::tools::{CurrentTool, EditorTool};
use crate::map::{MapData, PlacedItem, Selected};

use super::hit_detection::item_overlaps_rect;
use super::BoxSelectState;

#[allow(clippy::too_many_arguments)]
pub fn handle_box_select(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    camera: CameraParams,
    items_query: Query<(Entity, &Transform, &Sprite, &PlacedItem)>,
    selected_query: Query<Entity, With<Selected>>,
    mut box_select_state: ResMut<BoxSelectState>,
    mut contexts: EguiContexts,
    images: Res<Assets<Image>>,
    map_data: Res<MapData>,
    annotations: AnnotationQueries,
) {
    if current_tool.tool != EditorTool::Select {
        box_select_state.is_selecting = false;
        return;
    }

    if !box_select_state.is_selecting {
        return;
    }

    // Don't update if over UI
    if is_cursor_over_ui(&mut contexts) {
        return;
    }

    let Some(world_pos) = camera.cursor_world_pos() else {
        return;
    };

    // Update current position
    box_select_state.current_world = world_pos;

    // On release, select all items in the box
    if mouse_button.just_released(MouseButton::Left) {
        box_select_state.is_selecting = false;

        let rect_min = box_select_state.start_world.min(box_select_state.current_world);
        let rect_max = box_select_state.start_world.max(box_select_state.current_world);

        // Only process if we dragged a meaningful distance (not just a click)
        let drag_distance =
            (box_select_state.current_world - box_select_state.start_world).length();
        if drag_distance < 5.0 {
            return;
        }

        let ctrl_held =
            keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

        // If not holding ctrl, clear existing selection
        if !ctrl_held {
            for entity in selected_query.iter() {
                commands.entity(entity).remove::<Selected>();
            }
        }

        // Select all items that overlap with the selection rectangle
        // Filter by visible and unlocked layers
        for (entity, transform, sprite, placed_item) in items_query.iter() {
            // Check if layer is visible and unlocked
            let layer_selectable = map_data
                .layers
                .iter()
                .find(|ld| ld.layer_type == placed_item.layer)
                .map(|ld| ld.visible && !ld.locked)
                .unwrap_or(true);

            if !layer_selectable {
                continue;
            }

            if item_overlaps_rect(rect_min, rect_max, transform, sprite, &images) {
                if ctrl_held && selected_query.contains(entity) {
                    // Ctrl + box select: toggle (deselect if already selected)
                    commands.entity(entity).remove::<Selected>();
                } else {
                    commands.entity(entity).insert(Selected);
                }
            }
        }

        // Select annotations that overlap with the selection rectangle
        // Only if annotation layer is visible and not locked
        let annotation_selectable =
            is_annotation_layer_visible(&map_data) && !is_annotation_layer_locked(&map_data);

        if annotation_selectable {
            for (entity, path) in annotations.paths.iter() {
                if path_overlaps_rect(rect_min, rect_max, path) {
                    if ctrl_held && selected_query.contains(entity) {
                        commands.entity(entity).remove::<Selected>();
                    } else {
                        commands.entity(entity).insert(Selected);
                    }
                }
            }

            for (entity, line) in annotations.lines.iter() {
                if line_overlaps_rect(rect_min, rect_max, line) {
                    if ctrl_held && selected_query.contains(entity) {
                        commands.entity(entity).remove::<Selected>();
                    } else {
                        commands.entity(entity).insert(Selected);
                    }
                }
            }

            for (entity, transform, text) in annotations.texts.iter() {
                if text_overlaps_rect(rect_min, rect_max, transform, text) {
                    if ctrl_held && selected_query.contains(entity) {
                        commands.entity(entity).remove::<Selected>();
                    } else {
                        commands.entity(entity).insert(Selected);
                    }
                }
            }
        }
    }
}
