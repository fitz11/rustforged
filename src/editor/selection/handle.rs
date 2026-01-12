//! Selection handling - click to select, start dragging.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::editor::annotations::{
    is_annotation_layer_locked, is_annotation_layer_visible, AnnotationMarker, DrawnLine,
    DrawnPath, TextAnnotation,
};
use crate::editor::params::{is_cursor_over_ui, AnnotationQueries, CameraWithProjection};
use crate::editor::tools::{CurrentTool, EditorTool};
use crate::map::{MapData, PlacedItem, Selected};
use crate::session::{get_handle_at_position, LiveSessionState, ViewportDragMode};

use super::hit_detection::{
    check_rotation_handle_hit, compute_selection_bounds, find_clicked_annotation,
    get_selection_handle_at_position, get_sprite_half_size, point_in_item,
};
use super::{AnnotationDragData, BoxSelectState, DragState, SelectionDragMode};

#[allow(clippy::too_many_arguments)]
pub fn handle_selection(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    camera: CameraWithProjection,
    items_query: Query<(Entity, &Transform, &Sprite, &PlacedItem)>,
    selected_query: Query<Entity, With<Selected>>,
    selected_sprites_query: Query<(&Transform, &Sprite), With<Selected>>,
    mut drag_state: ResMut<DragState>,
    mut box_select_state: ResMut<BoxSelectState>,
    mut contexts: EguiContexts,
    images: Res<Assets<Image>>,
    map_data: Res<MapData>,
    session_state: Res<LiveSessionState>,
    annotations: AnnotationQueries,
) {
    if current_tool.tool != EditorTool::Select {
        return;
    }

    // Don't interact if over UI
    if is_cursor_over_ui(&mut contexts) {
        return;
    }

    let Some(world_pos) = camera.cursor_world_pos() else {
        return;
    };

    // Get camera scale for handle detection
    let camera_scale = camera.zoom_scale();

    // Check if clicking on a live session viewport handle FIRST
    // This prevents selecting items beneath the viewport gizmo
    if mouse_button.just_pressed(MouseButton::Left)
        && session_state.is_active
        && get_handle_at_position(world_pos, &session_state, camera_scale) != ViewportDragMode::None
    {
        return;
    }

    let ctrl_held =
        keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

    if mouse_button.just_pressed(MouseButton::Left) {
        // First, check if we clicked on a selection handle or inside the selection bounds
        // This takes priority over clicking on individual items
        // Check rotation handle first (it's per-item and accounts for rotation)
        let handle_mode = if check_rotation_handle_hit(
            world_pos,
            camera_scale,
            &selected_sprites_query,
            &images,
        ) {
            SelectionDragMode::Rotate
        } else {
            get_selection_handle_at_position(
                world_pos,
                &selected_sprites_query,
                &images,
                camera_scale,
            )
        };

        if handle_mode != SelectionDragMode::None && !ctrl_held {
            // Get bounds for resize operations (still needed for resize logic)
            let bounds = compute_selection_bounds(&selected_sprites_query, &images);
            // Start resize, move, or rotate operation
            start_selection_drag(
                &mut drag_state,
                world_pos,
                handle_mode,
                bounds,
                &selected_query,
                &items_query,
                &images,
                &annotations.paths,
                &annotations.lines,
                &annotations.texts,
            );
            return;
        }

        // Find what item (if any) we clicked on
        // Filter by visible and unlocked layers
        let mut items: Vec<_> = items_query
            .iter()
            .filter(|(_, _, _, placed_item)| {
                map_data
                    .layers
                    .iter()
                    .find(|ld| ld.layer_type == placed_item.layer)
                    .map(|ld| ld.visible && !ld.locked)
                    .unwrap_or(true)
            })
            .collect();
        items.sort_by(|a, b| b.1.translation.z.partial_cmp(&a.1.translation.z).unwrap());

        let clicked_item = items
            .iter()
            .find(|(_, transform, sprite, _)| point_in_item(world_pos, transform, sprite, &images));

        // Check annotations (they're on top, z=350, so check them first)
        // Only check if annotation layer is visible and not locked
        let annotation_selectable =
            is_annotation_layer_visible(&map_data) && !is_annotation_layer_locked(&map_data);

        let clicked_annotation = if annotation_selectable {
            find_clicked_annotation(
                world_pos,
                &annotations.paths,
                &annotations.lines,
                &annotations.texts,
            )
        } else {
            None
        };

        // Annotations are rendered above placed items, so prioritize them
        if let Some(entity) = clicked_annotation {
            let is_selected = selected_query.contains(entity);

            if ctrl_held {
                // Ctrl+click: toggle selection
                if is_selected {
                    commands.entity(entity).remove::<Selected>();
                } else {
                    commands.entity(entity).insert(Selected);
                }
            } else if is_selected {
                // Clicked on already selected annotation: start dragging all selected
                start_selection_drag(
                    &mut drag_state,
                    world_pos,
                    SelectionDragMode::Move,
                    None,
                    &selected_query,
                    &items_query,
                    &images,
                    &annotations.paths,
                    &annotations.lines,
                    &annotations.texts,
                );
            } else {
                // Clicked on unselected annotation: clear selection and select this one
                for entity in selected_query.iter() {
                    commands.entity(entity).remove::<Selected>();
                }
                commands.entity(entity).insert(Selected);

                // Start dragging this annotation
                start_drag_for_entity(
                    &mut drag_state,
                    world_pos,
                    entity,
                    &items_query,
                    &annotations.paths,
                    &annotations.lines,
                    &annotations.texts,
                );
            }
        } else if let Some(&(entity, transform, _, _)) = clicked_item {
            let is_selected = selected_query.contains(entity);

            if ctrl_held {
                // Ctrl+click: toggle selection
                if is_selected {
                    commands.entity(entity).remove::<Selected>();
                } else {
                    commands.entity(entity).insert(Selected);
                }
            } else if is_selected {
                // Clicked on already selected item: start dragging all selected items
                start_selection_drag(
                    &mut drag_state,
                    world_pos,
                    SelectionDragMode::Move,
                    None,
                    &selected_query,
                    &items_query,
                    &images,
                    &annotations.paths,
                    &annotations.lines,
                    &annotations.texts,
                );
            } else {
                // Clicked on unselected item: clear selection and select this one
                for entity in selected_query.iter() {
                    commands.entity(entity).remove::<Selected>();
                }
                commands.entity(entity).insert(Selected);

                // Start dragging this item
                drag_state.is_dragging = true;
                drag_state.mode = SelectionDragMode::Move;
                drag_state.drag_start_world = world_pos;
                drag_state.entity_start_positions =
                    vec![(entity, transform.translation.truncate())];
                drag_state.entity_start_scales = vec![(entity, transform.scale)];
                drag_state.annotation_drag_data.clear();
            }
        } else {
            // Clicked on empty space - start box selection
            if !ctrl_held {
                // Clear selection
                for entity in selected_query.iter() {
                    commands.entity(entity).remove::<Selected>();
                }
            }
            box_select_state.is_selecting = true;
            box_select_state.start_world = world_pos;
            box_select_state.current_world = world_pos;
        }
    }
}

/// Start dragging/resizing/rotating all selected entities
#[allow(clippy::too_many_arguments)]
fn start_selection_drag(
    drag_state: &mut ResMut<DragState>,
    world_pos: Vec2,
    mode: SelectionDragMode,
    bounds: Option<(Vec2, Vec2)>,
    selected_query: &Query<Entity, With<Selected>>,
    items_query: &Query<(Entity, &Transform, &Sprite, &PlacedItem)>,
    images: &Assets<Image>,
    paths_query: &Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    lines_query: &Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    texts_query: &Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) {
    drag_state.is_dragging = true;
    drag_state.mode = mode;
    drag_state.drag_start_world = world_pos;
    drag_state.original_bounds = bounds;
    drag_state.entity_start_positions.clear();
    drag_state.entity_start_scales.clear();
    drag_state.entity_start_rotations.clear();
    drag_state.entity_start_half_sizes.clear();
    drag_state.rotation_start_angle = None;
    drag_state.annotation_drag_data.clear();

    // For rotation, calculate the starting angle from selection center to cursor
    if mode == SelectionDragMode::Rotate
        && let Some((min, max)) = bounds
    {
        let center = (min + max) / 2.0;
        let angle = (world_pos - center).to_angle();
        drag_state.rotation_start_angle = Some(angle);
    }

    for entity in selected_query.iter() {
        // Check if it's a placed item
        if let Ok((_, t, sprite, _)) = items_query.get(entity) {
            drag_state
                .entity_start_positions
                .push((entity, t.translation.truncate()));
            drag_state.entity_start_scales.push((entity, t.scale));
            drag_state
                .entity_start_rotations
                .push((entity, t.rotation));
            // Store original half-size (sprite size * scale) for rotation-aware resizing
            let half_size = get_sprite_half_size(sprite, images) * t.scale.truncate();
            drag_state.entity_start_half_sizes.push((entity, half_size));
        }
        // Check if it's a path
        else if let Ok((_, path)) = paths_query.get(entity) {
            drag_state.annotation_drag_data.push((
                entity,
                AnnotationDragData::Path {
                    original_points: path.points.clone(),
                },
            ));
        }
        // Check if it's a line
        else if let Ok((_, line)) = lines_query.get(entity) {
            drag_state.annotation_drag_data.push((
                entity,
                AnnotationDragData::Line {
                    original_start: line.start,
                    original_end: line.end,
                },
            ));
        }
        // Check if it's a text annotation
        else if let Ok((_, t, _)) = texts_query.get(entity) {
            drag_state.annotation_drag_data.push((
                entity,
                AnnotationDragData::Text {
                    original_position: t.translation.truncate(),
                },
            ));
        }
    }
}

/// Start dragging a single entity
fn start_drag_for_entity(
    drag_state: &mut ResMut<DragState>,
    world_pos: Vec2,
    entity: Entity,
    items_query: &Query<(Entity, &Transform, &Sprite, &PlacedItem)>,
    paths_query: &Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    lines_query: &Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    texts_query: &Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) {
    drag_state.is_dragging = true;
    drag_state.mode = SelectionDragMode::Move;
    drag_state.drag_start_world = world_pos;
    drag_state.entity_start_positions.clear();
    drag_state.entity_start_scales.clear();
    drag_state.annotation_drag_data.clear();

    // Check if it's a placed item
    if let Ok((_, t, _, _)) = items_query.get(entity) {
        drag_state
            .entity_start_positions
            .push((entity, t.translation.truncate()));
        drag_state.entity_start_scales.push((entity, t.scale));
    }
    // Check if it's a path
    else if let Ok((_, path)) = paths_query.get(entity) {
        drag_state.annotation_drag_data.push((
            entity,
            AnnotationDragData::Path {
                original_points: path.points.clone(),
            },
        ));
    }
    // Check if it's a line
    else if let Ok((_, line)) = lines_query.get(entity) {
        drag_state.annotation_drag_data.push((
            entity,
            AnnotationDragData::Line {
                original_start: line.start,
                original_end: line.end,
            },
        ));
    }
    // Check if it's a text annotation
    else if let Ok((_, t, _)) = texts_query.get(entity) {
        drag_state.annotation_drag_data.push((
            entity,
            AnnotationDragData::Text {
                original_position: t.translation.truncate(),
            },
        ));
    }
}
