//! Drag operations - move, resize, and rotate.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::editor::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use crate::editor::params::{is_cursor_over_ui, CameraParams};
use crate::editor::tools::{CurrentTool, EditorTool};
use crate::map::{MapData, PlacedItem};

use super::{AnnotationDragData, DragState, SelectionDragMode, ROTATION_SNAP_INCREMENT};

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    camera: CameraParams,
    mut items_query: Query<&mut Transform, With<PlacedItem>>,
    mut drag_state: ResMut<DragState>,
    map_data: Res<MapData>,
    mut contexts: EguiContexts,
    // Mutable annotation queries for moving
    mut paths_query: Query<&mut DrawnPath, With<AnnotationMarker>>,
    mut lines_query: Query<&mut DrawnLine, With<AnnotationMarker>>,
    mut text_transforms_query: Query<
        &mut Transform,
        (With<TextAnnotation>, With<AnnotationMarker>, Without<PlacedItem>),
    >,
) {
    if current_tool.tool != EditorTool::Select {
        drag_state.is_dragging = false;
        drag_state.mode = SelectionDragMode::None;
        drag_state.original_bounds = None;
        return;
    }

    // Stop dragging on mouse release
    if mouse_button.just_released(MouseButton::Left) {
        drag_state.is_dragging = false;
        drag_state.mode = SelectionDragMode::None;
        drag_state.original_bounds = None;
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
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if shift_held && drag_state.mode == SelectionDragMode::Move {
        drag_offset.x = (drag_offset.x / map_data.grid_size).round() * map_data.grid_size;
        drag_offset.y = (drag_offset.y / map_data.grid_size).round() * map_data.grid_size;
    }

    match drag_state.mode {
        SelectionDragMode::Move => {
            // Apply offset to each placed item, maintaining relative positions
            for (entity, start_pos) in &drag_state.entity_start_positions {
                if let Ok(mut transform) = items_query.get_mut(*entity) {
                    let new_pos = *start_pos + drag_offset;
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
                        if let Ok(mut transform) = text_transforms_query.get_mut(*entity) {
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
            // Rotation-aware resize: transform mouse to each item's local space
            // and calculate scale/position based on local coordinates

            // Iterate through all selected placed items
            for (((entity, orig_pos), (_, orig_scale)), (_, orig_half_size)) in drag_state
                .entity_start_positions
                .iter()
                .zip(drag_state.entity_start_scales.iter())
                .zip(drag_state.entity_start_half_sizes.iter())
            {
                // Get the original rotation for this entity
                let orig_rotation = drag_state
                    .entity_start_rotations
                    .iter()
                    .find(|(e, _)| *e == *entity)
                    .map(|(_, r)| *r)
                    .unwrap_or(Quat::IDENTITY);

                let (angle, _, _) = orig_rotation.to_euler(EulerRot::ZYX);

                // Transform mouse position to item's local coordinate space
                // (undo the rotation around the item's original center)
                let world_diff = world_pos - *orig_pos;
                let cos_a = (-angle).cos();
                let sin_a = (-angle).sin();
                let local_mouse = Vec2::new(
                    world_diff.x * cos_a - world_diff.y * sin_a,
                    world_diff.x * sin_a + world_diff.y * cos_a,
                );

                // h = original half size in local space
                let h = *orig_half_size;

                // Calculate new bounds in local space based on resize mode
                // For each mode, one or more edges move to the mouse position,
                // while the opposite edge(s) stay fixed
                let (new_min_local, new_max_local) = match drag_state.mode {
                    SelectionDragMode::ResizeE => {
                        // E edge moves to mouse, W edge fixed at -h.x
                        (Vec2::new(-h.x, -h.y), Vec2::new(local_mouse.x, h.y))
                    }
                    SelectionDragMode::ResizeW => {
                        // W edge moves to mouse, E edge fixed at h.x
                        (Vec2::new(local_mouse.x, -h.y), Vec2::new(h.x, h.y))
                    }
                    SelectionDragMode::ResizeN => {
                        // N edge moves to mouse, S edge fixed at -h.y
                        (Vec2::new(-h.x, -h.y), Vec2::new(h.x, local_mouse.y))
                    }
                    SelectionDragMode::ResizeS => {
                        // S edge moves to mouse, N edge fixed at h.y
                        (Vec2::new(-h.x, local_mouse.y), Vec2::new(h.x, h.y))
                    }
                    SelectionDragMode::ResizeNE => {
                        // NE corner moves to mouse, SW corner fixed
                        (Vec2::new(-h.x, -h.y), local_mouse)
                    }
                    SelectionDragMode::ResizeSW => {
                        // SW corner moves to mouse, NE corner fixed
                        (local_mouse, Vec2::new(h.x, h.y))
                    }
                    SelectionDragMode::ResizeNW => {
                        // NW corner moves to mouse, SE corner fixed
                        (Vec2::new(local_mouse.x, -h.y), Vec2::new(h.x, local_mouse.y))
                    }
                    SelectionDragMode::ResizeSE => {
                        // SE corner moves to mouse, NW corner fixed
                        (Vec2::new(-h.x, local_mouse.y), Vec2::new(local_mouse.x, h.y))
                    }
                    _ => (Vec2::new(-h.x, -h.y), Vec2::new(h.x, h.y)),
                };

                // Calculate new size and center in local space
                let new_size_local = new_max_local - new_min_local;
                let mut new_center_local = (new_min_local + new_max_local) / 2.0;

                // Calculate scale factors (avoid division by zero, ensure positive)
                let orig_size_local = h * 2.0;
                let mut scale_x = if orig_size_local.x.abs() > 0.001 {
                    (new_size_local.x / orig_size_local.x).abs().max(0.01)
                } else {
                    1.0
                };
                let mut scale_y = if orig_size_local.y.abs() > 0.001 {
                    (new_size_local.y / orig_size_local.y).abs().max(0.01)
                } else {
                    1.0
                };

                // Hold shift to maintain current aspect ratio during resize
                if shift_held {
                    // Determine which scale to use based on resize direction
                    let uniform = match drag_state.mode {
                        // Horizontal handles: use horizontal scale
                        SelectionDragMode::ResizeE | SelectionDragMode::ResizeW => scale_x,
                        // Vertical handles: use vertical scale
                        SelectionDragMode::ResizeN | SelectionDragMode::ResizeS => scale_y,
                        // Corner handles: use the larger scale to preserve largest dimension
                        _ => scale_x.max(scale_y),
                    };
                    scale_x = uniform;
                    scale_y = uniform;

                    // For corner handles, recalculate center to keep opposite corner fixed
                    // The opposite corner should stay at its original position
                    new_center_local = match drag_state.mode {
                        SelectionDragMode::ResizeNE => {
                            // SW corner fixed at (-h.x, -h.y)
                            Vec2::new(h.x * (uniform - 1.0), h.y * (uniform - 1.0))
                        }
                        SelectionDragMode::ResizeSW => {
                            // NE corner fixed at (h.x, h.y)
                            Vec2::new(-h.x * (uniform - 1.0), -h.y * (uniform - 1.0))
                        }
                        SelectionDragMode::ResizeNW => {
                            // SE corner fixed at (h.x, -h.y)
                            Vec2::new(-h.x * (uniform - 1.0), h.y * (uniform - 1.0))
                        }
                        SelectionDragMode::ResizeSE => {
                            // NW corner fixed at (-h.x, h.y)
                            Vec2::new(h.x * (uniform - 1.0), -h.y * (uniform - 1.0))
                        }
                        _ => new_center_local,
                    };
                }

                // Transform the new center offset back to world space
                let cos_a = angle.cos();
                let sin_a = angle.sin();
                let world_center_offset = Vec2::new(
                    new_center_local.x * cos_a - new_center_local.y * sin_a,
                    new_center_local.x * sin_a + new_center_local.y * cos_a,
                );

                if let Ok(mut transform) = items_query.get_mut(*entity) {
                    // Apply new position (original position + rotated center offset)
                    let new_pos = *orig_pos + world_center_offset;
                    transform.translation.x = new_pos.x;
                    transform.translation.y = new_pos.y;
                    transform.scale.x = orig_scale.x * scale_x;
                    transform.scale.y = orig_scale.y * scale_y;
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
            for (entity, original_rotation) in &drag_state.entity_start_rotations {
                if let Ok(mut transform) = items_query.get_mut(*entity) {
                    transform.rotation = *original_rotation * Quat::from_rotation_z(angle_delta);
                }
            }
        }
        SelectionDragMode::None => {}
    }
}
