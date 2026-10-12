//! Drag operations - move, resize, and rotate.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::editor::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use crate::editor::history::{EditorCommand, MapObject, RecordEditorCommand};
use crate::editor::params::{CameraParams, is_cursor_over_ui};
use crate::input::shift_held;
use crate::map::{ItemId, MapData, PlacedItem};

use super::hit_detection::rotate_point;
use super::resize::{resize_center_offset, resize_factors};
use super::{AnnotationDragData, DragState, ROTATION_SNAP_INCREMENT, SelectionDragMode};

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera: CameraParams,
    mut items_query: Query<(&ItemId, &PlacedItem, &mut Transform)>,
    mut drag_state: ResMut<DragState>,
    map_data: Res<MapData>,
    mut contexts: EguiContexts,
    mut history_writer: MessageWriter<RecordEditorCommand>,
    // Mutable annotation queries for moving
    mut paths_query: Query<&mut DrawnPath, With<AnnotationMarker>>,
    mut lines_query: Query<&mut DrawnLine, With<AnnotationMarker>>,
    mut text_transforms_query: Query<
        (&mut Transform, &TextAnnotation),
        (With<AnnotationMarker>, Without<PlacedItem>),
    >,
    annotation_ids: Query<&ItemId, With<AnnotationMarker>>,
) {
    // Stop dragging on mouse release, recording the net change to the placed items and
    // annotations as one undo step (covers move/resize/rotate)
    if mouse_button.just_released(MouseButton::Left) {
        let items = drag_state.items.iter().filter_map(|start| {
            let (id, item, current) = items_query.get(start.entity).ok()?;
            Some((
                MapObject::item(*id, item.clone(), start.transform),
                MapObject::item(*id, item.clone(), *current),
            ))
        });
        let annotations = drag_state
            .annotation_drag_data
            .iter()
            .filter_map(|(entity, start)| {
                let id = *annotation_ids.get(*entity).ok()?;
                match start {
                    AnnotationDragData::Path { original_points } => {
                        let path = paths_query.get(*entity).ok()?;
                        let before = DrawnPath {
                            points: original_points.clone(),
                            ..path.clone()
                        };
                        Some((
                            MapObject::Path { id, path: before },
                            MapObject::Path {
                                id,
                                path: path.clone(),
                            },
                        ))
                    }
                    AnnotationDragData::Line {
                        original_start,
                        original_end,
                    } => {
                        let line = lines_query.get(*entity).ok()?;
                        let before = DrawnLine {
                            start: *original_start,
                            end: *original_end,
                            ..line.clone()
                        };
                        Some((
                            MapObject::Line { id, line: before },
                            MapObject::Line {
                                id,
                                line: line.clone(),
                            },
                        ))
                    }
                    AnnotationDragData::Text { original_position } => {
                        let (transform, text) = text_transforms_query.get(*entity).ok()?;
                        Some((
                            MapObject::Text {
                                id,
                                text: text.clone(),
                                position: *original_position,
                            },
                            MapObject::Text {
                                id,
                                text: text.clone(),
                                position: transform.translation.truncate(),
                            },
                        ))
                    }
                }
            });
        if let Some(command) = EditorCommand::change(items.chain(annotations)) {
            history_writer.write(RecordEditorCommand { command });
        }

        drag_state.is_dragging = false;
        drag_state.mode = SelectionDragMode::None;
        drag_state.original_bounds = None;
        drag_state.items.clear();
        return;
    }

    if !drag_state.is_dragging {
        return;
    }

    // Don't drag if over UI
    if is_cursor_over_ui(&mut contexts) {
        return;
    }

    let Some(world_pos) = camera.cursor_world_pos() else {
        return;
    };

    // Calculate drag offset
    let mut drag_offset = world_pos - drag_state.drag_start_world;

    // Shift = snap the offset to grid increments (for move mode)
    let shift_held = shift_held(&keyboard);
    if shift_held && drag_state.mode == SelectionDragMode::Move {
        drag_offset.x = (drag_offset.x / map_data.grid_size).round() * map_data.grid_size;
        drag_offset.y = (drag_offset.y / map_data.grid_size).round() * map_data.grid_size;
    }

    match drag_state.mode {
        SelectionDragMode::Move => {
            // Apply offset to each placed item, maintaining relative positions
            for start in &drag_state.items {
                if let Ok((_, _, mut transform)) = items_query.get_mut(start.entity) {
                    let new_pos = start.transform.translation.truncate() + drag_offset;
                    transform.translation.x = new_pos.x;
                    transform.translation.y = new_pos.y;
                }
            }

            // Apply offset to annotations
            for (entity, drag_data) in &drag_state.annotation_drag_data {
                match drag_data {
                    AnnotationDragData::Path { original_points } => {
                        if let Ok(mut path) = paths_query.get_mut(*entity) {
                            path.points =
                                original_points.iter().map(|p| *p + drag_offset).collect();
                        }
                    }
                    AnnotationDragData::Line {
                        original_start,
                        original_end,
                    } => {
                        if let Ok(mut line) = lines_query.get_mut(*entity) {
                            line.start = *original_start + drag_offset;
                            line.end = *original_end + drag_offset;
                        }
                    }
                    AnnotationDragData::Text { original_position } => {
                        if let Ok((mut transform, _)) = text_transforms_query.get_mut(*entity) {
                            let new_pos = *original_position + drag_offset;
                            transform.translation.x = new_pos.x;
                            transform.translation.y = new_pos.y;
                        }
                    }
                }
            }
        }
        SelectionDragMode::ResizeN
        | SelectionDragMode::ResizeS
        | SelectionDragMode::ResizeE
        | SelectionDragMode::ResizeW
        | SelectionDragMode::ResizeNE
        | SelectionDragMode::ResizeNW
        | SelectionDragMode::ResizeSE
        | SelectionDragMode::ResizeSW => {
            // The grabbed item's handle follows the mouse (in the item's own frame, so
            // rotation is accounted for); every selected item scales by the same factors,
            // each with its own opposite edges held still
            let Some(reference) = drag_state
                .resize_reference
                .and_then(|entity| drag_state.items.iter().find(|s| s.entity == entity))
            else {
                return;
            };
            let center = reference.transform.translation.truncate();
            let (angle, _, _) = reference.transform.rotation.to_euler(EulerRot::ZYX);
            let local_mouse = rotate_point(world_pos, center, -angle) - center;
            let factors = resize_factors(
                drag_state.mode,
                reference.half_size,
                local_mouse,
                shift_held,
            );

            for start in &drag_state.items {
                let (angle, _, _) = start.transform.rotation.to_euler(EulerRot::ZYX);
                let local_offset = resize_center_offset(drag_state.mode, start.half_size, factors);
                let offset = rotate_point(local_offset, Vec2::ZERO, angle);
                if let Ok((_, _, mut transform)) = items_query.get_mut(start.entity) {
                    let new_pos = start.transform.translation.truncate() + offset;
                    transform.translation.x = new_pos.x;
                    transform.translation.y = new_pos.y;
                    transform.scale.x = start.transform.scale.x * factors.x;
                    transform.scale.y = start.transform.scale.y * factors.y;
                }
            }
        }
        SelectionDragMode::Rotate => {
            // Get original bounds and start angle - required for rotation
            let Some((orig_min, orig_max)) = drag_state.original_bounds else {
                return;
            };
            let Some(start_angle) = drag_state.rotation_start_angle else {
                return;
            };

            let center = (orig_min + orig_max) / 2.0;
            let current_angle = (world_pos - center).to_angle();
            let mut angle_delta = current_angle - start_angle;

            // Shift = snap to 15° increments
            if shift_held {
                let snap_rad = ROTATION_SNAP_INCREMENT.to_radians();
                angle_delta = (angle_delta / snap_rad).round() * snap_rad;
            }

            // Apply rotation to each entity around its own center
            for start in &drag_state.items {
                if let Ok((_, _, mut transform)) = items_query.get_mut(start.entity) {
                    transform.rotation =
                        start.transform.rotation * Quat::from_rotation_z(angle_delta);
                }
            }
        }
        SelectionDragMode::None => {}
    }
}
