use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use bevy_egui::EguiContexts;

use crate::map::{MapData, PlacedItem, Selected};
use crate::session::{get_handle_at_position, LiveSessionState, ViewportDragMode};

use super::annotations::{
    is_annotation_layer_locked, is_annotation_layer_visible, line_bounds, line_overlaps_rect,
    path_bounds, path_overlaps_rect, point_in_text, point_near_line, point_near_path, text_bounds,
    text_overlaps_rect, AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation,
};
use super::tools::{CurrentTool, EditorTool};
use super::EditorCamera;

/// Information about an annotation's original state when dragging started
#[derive(Clone)]
pub enum AnnotationDragData {
    Path { original_points: Vec<Vec2> },
    Line { original_start: Vec2, original_end: Vec2 },
    Text { original_position: Vec2 },
}

/// Drag mode for selection interaction (move or resize)
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectionDragMode {
    #[default]
    None,
    Move,
    ResizeN,
    ResizeS,
    ResizeE,
    ResizeW,
    ResizeNE,
    ResizeNW,
    ResizeSE,
    ResizeSW,
}

impl SelectionDragMode {
    /// Get the appropriate cursor icon for this drag mode
    pub fn cursor_icon(&self) -> Option<CursorIcon> {
        match self {
            SelectionDragMode::None => None,
            SelectionDragMode::Move => Some(CursorIcon::System(SystemCursorIcon::Move)),
            SelectionDragMode::ResizeN | SelectionDragMode::ResizeS => {
                Some(CursorIcon::System(SystemCursorIcon::NsResize))
            }
            SelectionDragMode::ResizeE | SelectionDragMode::ResizeW => {
                Some(CursorIcon::System(SystemCursorIcon::EwResize))
            }
            SelectionDragMode::ResizeNE | SelectionDragMode::ResizeSW => {
                Some(CursorIcon::System(SystemCursorIcon::NeswResize))
            }
            SelectionDragMode::ResizeNW | SelectionDragMode::ResizeSE => {
                Some(CursorIcon::System(SystemCursorIcon::NwseResize))
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct DragState {
    pub is_dragging: bool,
    pub drag_start_world: Vec2,
    /// The current drag mode (move or resize direction)
    pub mode: SelectionDragMode,
    /// Original selection bounds when resize started (min, max)
    pub original_bounds: Option<(Vec2, Vec2)>,
    /// Maps entity to its starting position when drag began (for PlacedItems)
    pub entity_start_positions: Vec<(Entity, Vec2)>,
    /// Maps entity to its original scale when drag began (for resizing)
    pub entity_start_scales: Vec<(Entity, Vec3)>,
    /// Maps entity to its annotation drag data when drag began
    pub annotation_drag_data: Vec<(Entity, AnnotationDragData)>,
}

#[derive(Resource, Default)]
pub struct BoxSelectState {
    pub is_selecting: bool,
    pub start_world: Vec2,
    pub current_world: Vec2,
}

/// Bundled annotation queries to reduce system parameter count
#[derive(SystemParam)]
pub struct AnnotationQueries<'w, 's> {
    pub paths: Query<'w, 's, (Entity, &'static DrawnPath), With<AnnotationMarker>>,
    pub lines: Query<'w, 's, (Entity, &'static DrawnLine), With<AnnotationMarker>>,
    pub texts: Query<'w, 's, (Entity, &'static Transform, &'static TextAnnotation), With<AnnotationMarker>>,
}

/// Get the half-size of a sprite, accounting for custom_size or image dimensions
fn get_sprite_half_size(
    sprite: &Sprite,
    images: &Assets<Image>,
) -> Vec2 {
    // Check for custom_size first
    if let Some(custom_size) = sprite.custom_size {
        return custom_size / 2.0;
    }

    // Try to get size from the image
    if let Some(image) = images.get(&sprite.image) {
        let size = image.size().as_vec2();
        return size / 2.0;
    }

    // Fallback to a default size
    Vec2::splat(32.0)
}

/// Check if a point is inside an item's bounds
fn point_in_item(
    world_pos: Vec2,
    transform: &Transform,
    sprite: &Sprite,
    images: &Assets<Image>,
) -> bool {
    let item_pos = transform.translation.truncate();
    let half_size = get_sprite_half_size(sprite, images) * transform.scale.truncate();
    let diff = world_pos - item_pos;
    diff.x.abs() < half_size.x && diff.y.abs() < half_size.y
}

/// Check if an item overlaps with a rectangle (defined by two corners)
fn item_overlaps_rect(
    rect_min: Vec2,
    rect_max: Vec2,
    transform: &Transform,
    sprite: &Sprite,
    images: &Assets<Image>,
) -> bool {
    let item_pos = transform.translation.truncate();
    let half_size = get_sprite_half_size(sprite, images) * transform.scale.truncate();

    let item_min = item_pos - half_size;
    let item_max = item_pos + half_size;

    // Check for overlap (AABB intersection)
    rect_min.x < item_max.x
        && rect_max.x > item_min.x
        && rect_min.y < item_max.y
        && rect_max.y > item_min.y
}

/// Handle size for resize handles (in world units, will be scaled by camera)
const HANDLE_SIZE: f32 = 8.0;

/// Compute the combined bounding box for all selected placed items
pub fn compute_selection_bounds(
    selected_query: &Query<(&Transform, &Sprite), With<Selected>>,
    images: &Assets<Image>,
) -> Option<(Vec2, Vec2)> {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    let mut found_any = false;

    for (transform, sprite) in selected_query.iter() {
        let pos = transform.translation.truncate();
        let half_size = get_sprite_half_size(sprite, images) * transform.scale.truncate();
        let item_min = pos - half_size;
        let item_max = pos + half_size;

        min = min.min(item_min);
        max = max.max(item_max);
        found_any = true;
    }

    if found_any {
        Some((min, max))
    } else {
        None
    }
}

/// Determine which handle (if any) is under the cursor for selected items
pub fn get_selection_handle_at_position(
    world_pos: Vec2,
    bounds: (Vec2, Vec2),
    camera_scale: f32,
) -> SelectionDragMode {
    let (min, max) = bounds;
    let center = (min + max) / 2.0;

    // Adjust handle hit area based on camera zoom
    let hit_size = HANDLE_SIZE * camera_scale * 1.5;

    // Check corners first (higher priority)
    let corners = [
        (Vec2::new(min.x, min.y), SelectionDragMode::ResizeSW),
        (Vec2::new(max.x, min.y), SelectionDragMode::ResizeSE),
        (Vec2::new(max.x, max.y), SelectionDragMode::ResizeNE),
        (Vec2::new(min.x, max.y), SelectionDragMode::ResizeNW),
    ];

    for (corner, mode) in corners {
        if (world_pos - corner).length() < hit_size {
            return mode;
        }
    }

    // Check edge handles
    let edges = [
        (Vec2::new(center.x, min.y), SelectionDragMode::ResizeS),
        (Vec2::new(center.x, max.y), SelectionDragMode::ResizeN),
        (Vec2::new(min.x, center.y), SelectionDragMode::ResizeW),
        (Vec2::new(max.x, center.y), SelectionDragMode::ResizeE),
    ];

    for (edge, mode) in edges {
        if (world_pos - edge).length() < hit_size {
            return mode;
        }
    }

    // Check if inside the selection rectangle (for move/grab)
    if world_pos.x >= min.x && world_pos.x <= max.x && world_pos.y >= min.y && world_pos.y <= max.y
    {
        return SelectionDragMode::Move;
    }

    SelectionDragMode::None
}

pub fn handle_selection(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform, &Projection), With<EditorCamera>>,
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
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.is_pointer_over_area()
    {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((camera, camera_transform, projection)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Get camera scale for handle detection
    let camera_scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };

    let ctrl_held =
        keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

    if mouse_button.just_pressed(MouseButton::Left) {
        // First, check if we clicked on a selection handle or inside the selection bounds
        // This takes priority over clicking on individual items
        if let Some(bounds) = compute_selection_bounds(&selected_sprites_query, &images) {
            let handle_mode = get_selection_handle_at_position(world_pos, bounds, camera_scale);

            if handle_mode != SelectionDragMode::None && !ctrl_held {
                // Start resize or move operation
                start_selection_drag(
                    &mut drag_state,
                    world_pos,
                    handle_mode,
                    Some(bounds),
                    &selected_query,
                    &items_query,
                    &annotations.paths,
                    &annotations.lines,
                    &annotations.texts,
                );
                return;
            }
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
            find_clicked_annotation(world_pos, &annotations.paths, &annotations.lines, &annotations.texts)
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
            // Clicked on empty space - but check if we clicked on a viewport handle
            let on_viewport_handle = session_state.is_active
                && get_handle_at_position(world_pos, &session_state, camera_scale)
                    != ViewportDragMode::None;

            if on_viewport_handle {
                // Don't start box selection if clicking on viewport handles
                return;
            }

            if !ctrl_held {
                // Clear selection
                for entity in selected_query.iter() {
                    commands.entity(entity).remove::<Selected>();
                }
            }
            // Start box selection
            box_select_state.is_selecting = true;
            box_select_state.start_world = world_pos;
            box_select_state.current_world = world_pos;
        }
    }
}

/// Find which annotation (if any) was clicked
fn find_clicked_annotation(
    world_pos: Vec2,
    paths_query: &Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    lines_query: &Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    texts_query: &Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) -> Option<Entity> {
    // Check text annotations first (they have clear bounds)
    for (entity, transform, text) in texts_query.iter() {
        if point_in_text(world_pos, transform, text) {
            return Some(entity);
        }
    }

    // Check lines
    for (entity, line) in lines_query.iter() {
        if point_near_line(world_pos, line) {
            return Some(entity);
        }
    }

    // Check paths
    for (entity, path) in paths_query.iter() {
        if point_near_path(world_pos, path) {
            return Some(entity);
        }
    }

    None
}

/// Start dragging/resizing all selected entities
fn start_selection_drag(
    drag_state: &mut ResMut<DragState>,
    world_pos: Vec2,
    mode: SelectionDragMode,
    bounds: Option<(Vec2, Vec2)>,
    selected_query: &Query<Entity, With<Selected>>,
    items_query: &Query<(Entity, &Transform, &Sprite, &PlacedItem)>,
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
    drag_state.annotation_drag_data.clear();

    for entity in selected_query.iter() {
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

pub fn handle_box_select(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    items_query: Query<(Entity, &Transform, &Sprite, &PlacedItem)>,
    selected_query: Query<Entity, With<Selected>>,
    mut box_select_state: ResMut<BoxSelectState>,
    mut contexts: EguiContexts,
    images: Res<Assets<Image>>,
    map_data: Res<MapData>,
    // Annotation queries
    paths_query: Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    lines_query: Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    texts_query: Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) {
    if current_tool.tool != EditorTool::Select {
        box_select_state.is_selecting = false;
        return;
    }

    if !box_select_state.is_selecting {
        return;
    }

    // Don't update if over UI
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.is_pointer_over_area()
    {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
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
            for (entity, path) in paths_query.iter() {
                if path_overlaps_rect(rect_min, rect_max, path) {
                    if ctrl_held && selected_query.contains(entity) {
                        commands.entity(entity).remove::<Selected>();
                    } else {
                        commands.entity(entity).insert(Selected);
                    }
                }
            }

            for (entity, line) in lines_query.iter() {
                if line_overlaps_rect(rect_min, rect_max, line) {
                    if ctrl_held && selected_query.contains(entity) {
                        commands.entity(entity).remove::<Selected>();
                    } else {
                        commands.entity(entity).insert(Selected);
                    }
                }
            }

            for (entity, transform, text) in texts_query.iter() {
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

pub fn handle_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut items_query: Query<&mut Transform, With<PlacedItem>>,
    mut drag_state: ResMut<DragState>,
    map_data: Res<MapData>,
    mut contexts: EguiContexts,
    // Annotation queries for moving
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
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.is_pointer_over_area()
    {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
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
                            path.points = original_points.iter().map(|p| *p + drag_offset).collect();
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
            // Get original bounds - required for resize
            let Some((orig_min, orig_max)) = drag_state.original_bounds else {
                return;
            };

            let orig_size = orig_max - orig_min;
            let orig_center = (orig_min + orig_max) / 2.0;

            // Calculate new bounds based on which edge is being dragged to mouse position
            let (new_min, new_max) = match drag_state.mode {
                SelectionDragMode::ResizeE => (orig_min, Vec2::new(world_pos.x, orig_max.y)),
                SelectionDragMode::ResizeW => (Vec2::new(world_pos.x, orig_min.y), orig_max),
                SelectionDragMode::ResizeN => (orig_min, Vec2::new(orig_max.x, world_pos.y)),
                SelectionDragMode::ResizeS => (Vec2::new(orig_min.x, world_pos.y), orig_max),
                SelectionDragMode::ResizeNE => (orig_min, world_pos),
                SelectionDragMode::ResizeSW => (world_pos, orig_max),
                SelectionDragMode::ResizeNW => {
                    (Vec2::new(world_pos.x, orig_min.y), Vec2::new(orig_max.x, world_pos.y))
                }
                SelectionDragMode::ResizeSE => {
                    (Vec2::new(orig_min.x, world_pos.y), Vec2::new(world_pos.x, orig_max.y))
                }
                _ => (orig_min, orig_max),
            };

            // Calculate new size and center
            let new_size = new_max - new_min;
            let new_center = (new_min + new_max) / 2.0;

            // Calculate scale factors (avoid division by zero)
            let scale_x = if orig_size.x.abs() > 0.001 {
                (new_size.x / orig_size.x).abs().max(0.01)
            } else {
                1.0
            };
            let scale_y = if orig_size.y.abs() > 0.001 {
                (new_size.y / orig_size.y).abs().max(0.01)
            } else {
                1.0
            };

            // Hold shift for uniform scaling on edge handles
            let (final_scale_x, final_scale_y) = if shift_held
                && matches!(
                    drag_state.mode,
                    SelectionDragMode::ResizeN
                        | SelectionDragMode::ResizeS
                        | SelectionDragMode::ResizeE
                        | SelectionDragMode::ResizeW
                )
            {
                let uniform = scale_x.max(scale_y);
                (uniform, uniform)
            } else {
                (scale_x, scale_y)
            };

            // Calculate center offset
            let center_offset = new_center - orig_center;

            // Apply scale and position to each placed item
            for ((entity, orig_pos), (_, orig_scale)) in drag_state
                .entity_start_positions
                .iter()
                .zip(drag_state.entity_start_scales.iter())
            {
                if let Ok(mut transform) = items_query.get_mut(*entity) {
                    // Scale relative to original center
                    let rel_pos = *orig_pos - orig_center;
                    let scaled_rel_pos =
                        Vec2::new(rel_pos.x * final_scale_x, rel_pos.y * final_scale_y);
                    let new_pos = orig_center + scaled_rel_pos + center_offset;

                    transform.translation.x = new_pos.x;
                    transform.translation.y = new_pos.y;
                    transform.scale.x = orig_scale.x * final_scale_x;
                    transform.scale.y = orig_scale.y * final_scale_y;
                }
            }
        }
        SelectionDragMode::None => {}
    }
}

pub fn draw_selection_indicators(
    mut gizmos: Gizmos,
    selected_sprites_query: Query<(&Transform, &Sprite), With<Selected>>,
    images: Res<Assets<Image>>,
    map_data: Res<MapData>,
    // Annotation queries
    selected_paths_query: Query<&DrawnPath, (With<Selected>, With<AnnotationMarker>)>,
    selected_lines_query: Query<&DrawnLine, (With<Selected>, With<AnnotationMarker>)>,
    selected_texts_query: Query<
        (&Transform, &TextAnnotation),
        (With<Selected>, With<AnnotationMarker>),
    >,
) {
    let selection_color = Color::srgb(0.2, 0.6, 1.0);
    let annotation_visible = is_annotation_layer_visible(&map_data);

    // Draw selection for sprites (placed items)
    for (transform, sprite) in selected_sprites_query.iter() {
        let pos = transform.translation.truncate();
        let scale = transform.scale.truncate();
        let half_size = get_sprite_half_size(sprite, &images);
        let scaled_half = half_size * scale;

        // Draw selection rectangle
        gizmos.rect_2d(
            Isometry2d::from_translation(pos),
            scaled_half * 2.0,
            selection_color,
        );

        // Draw corner handles (larger)
        let corner_handle_size = 4.0;
        let corners = [
            pos + Vec2::new(-scaled_half.x, -scaled_half.y),
            pos + Vec2::new(scaled_half.x, -scaled_half.y),
            pos + Vec2::new(scaled_half.x, scaled_half.y),
            pos + Vec2::new(-scaled_half.x, scaled_half.y),
        ];

        for corner in corners {
            gizmos.rect_2d(
                Isometry2d::from_translation(corner),
                Vec2::splat(corner_handle_size * 2.0),
                selection_color,
            );
        }

        // Draw edge handles (smaller)
        let edge_handle_size = 3.0;
        let edges = [
            pos + Vec2::new(0.0, -scaled_half.y), // S
            pos + Vec2::new(0.0, scaled_half.y),  // N
            pos + Vec2::new(-scaled_half.x, 0.0), // W
            pos + Vec2::new(scaled_half.x, 0.0),  // E
        ];

        for edge in edges {
            gizmos.rect_2d(
                Isometry2d::from_translation(edge),
                Vec2::splat(edge_handle_size * 2.0),
                selection_color,
            );
        }
    }

    // Only draw annotation selections if the layer is visible
    if !annotation_visible {
        return;
    }

    // Draw selection for paths
    for path in selected_paths_query.iter() {
        let (min, max) = path_bounds(path);
        let center = (min + max) / 2.0;
        let size = max - min;

        gizmos.rect_2d(Isometry2d::from_translation(center), size, selection_color);

        // Draw corner handles
        let handle_size = 4.0;
        let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];

        for corner in corners {
            gizmos.rect_2d(
                Isometry2d::from_translation(corner),
                Vec2::splat(handle_size * 2.0),
                selection_color,
            );
        }
    }

    // Draw selection for lines
    for line in selected_lines_query.iter() {
        let (min, max) = line_bounds(line);
        let center = (min + max) / 2.0;
        let size = max - min;

        // Lines might be very thin in one dimension, ensure minimum size for visibility
        let size = size.max(Vec2::splat(10.0));

        gizmos.rect_2d(Isometry2d::from_translation(center), size, selection_color);

        // Draw handles at line endpoints
        let handle_size = 4.0;
        gizmos.rect_2d(
            Isometry2d::from_translation(line.start),
            Vec2::splat(handle_size * 2.0),
            selection_color,
        );
        gizmos.rect_2d(
            Isometry2d::from_translation(line.end),
            Vec2::splat(handle_size * 2.0),
            selection_color,
        );
    }

    // Draw selection for text annotations
    for (transform, text) in selected_texts_query.iter() {
        let (min, max) = text_bounds(transform, text);
        let center = (min + max) / 2.0;
        let size = max - min;

        gizmos.rect_2d(Isometry2d::from_translation(center), size, selection_color);

        // Draw corner handles
        let handle_size = 4.0;
        let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];

        for corner in corners {
            gizmos.rect_2d(
                Isometry2d::from_translation(corner),
                Vec2::splat(handle_size * 2.0),
                selection_color,
            );
        }
    }
}

pub fn draw_box_select_rect(
    mut gizmos: Gizmos,
    box_select_state: Res<BoxSelectState>,
) {
    if !box_select_state.is_selecting {
        return;
    }

    let box_color = Color::srgba(0.2, 0.6, 1.0, 0.8);
    let fill_color = Color::srgba(0.2, 0.6, 1.0, 0.1);

    let start = box_select_state.start_world;
    let current = box_select_state.current_world;

    let center = (start + current) / 2.0;
    let size = (current - start).abs();

    // Draw the selection box outline
    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        size,
        box_color,
    );

    // Draw a semi-transparent fill (using multiple lines for visual effect)
    // Since gizmos don't have filled rectangles, we draw dashed inner lines
    let min = start.min(current);
    let max = start.max(current);
    let step = 10.0;

    // Horizontal lines
    let mut y = min.y + step;
    while y < max.y {
        gizmos.line_2d(Vec2::new(min.x, y), Vec2::new(max.x, y), fill_color);
        y += step;
    }
}

pub fn handle_fit_to_grid(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected_query: Query<(&mut Transform, &Sprite), With<Selected>>,
    map_data: Res<MapData>,
    images: Res<Assets<Image>>,
    mut contexts: EguiContexts,
) {
    // Don't trigger if typing in UI
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    // G = fit to grid
    if !keyboard.just_pressed(KeyCode::KeyG) {
        return;
    }

    for (mut transform, sprite) in selected_query.iter_mut() {
        let original_size = get_sprite_half_size(sprite, &images) * 2.0;

        if original_size.x > 0.0 && original_size.y > 0.0 {
            // Calculate scale to fit into one grid cell
            let grid_size = map_data.grid_size;
            let scale_x = grid_size / original_size.x;
            let scale_y = grid_size / original_size.y;

            // Use uniform scaling (the smaller of the two to fit within the cell)
            let uniform_scale = scale_x.min(scale_y);
            transform.scale = Vec3::new(uniform_scale, uniform_scale, 1.0);
        }
    }
}

pub fn handle_deletion(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_query: Query<Entity, With<Selected>>,
    mut contexts: EguiContexts,
) {
    // Don't trigger if typing in UI
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    let should_delete =
        keyboard.just_pressed(KeyCode::Delete) || keyboard.just_pressed(KeyCode::Backspace);

    if should_delete {
        for entity in selected_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Update cursor icon based on hover over selection handles
pub fn update_selection_cursor(
    current_tool: Res<CurrentTool>,
    window_query: Query<(Entity, &Window), With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform, &Projection), With<EditorCamera>>,
    selected_sprites_query: Query<(&Transform, &Sprite), With<Selected>>,
    drag_state: Res<DragState>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    mut contexts: EguiContexts,
) {
    // Only apply for select tool
    if current_tool.tool != EditorTool::Select {
        return;
    }

    let Ok((window_entity, window)) = window_query.single() else {
        return;
    };

    // Use default cursor over UI
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.is_pointer_over_area()
    {
        commands
            .entity(window_entity)
            .insert(CursorIcon::System(SystemCursorIcon::Default));
        return;
    }

    let Ok((camera, camera_transform, projection)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Get camera scale for handle detection
    let camera_scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };

    // If we're actively dragging, use the drag mode's cursor
    if drag_state.is_dragging
        && let Some(cursor) = drag_state.mode.cursor_icon()
    {
        commands.entity(window_entity).insert(cursor);
        return;
    }

    // Check if hovering over a selection handle
    if let Some(bounds) = compute_selection_bounds(&selected_sprites_query, &images) {
        let hover_mode = get_selection_handle_at_position(world_pos, bounds, camera_scale);

        if let Some(cursor) = hover_mode.cursor_icon() {
            commands.entity(window_entity).insert(cursor);
            return;
        }
    }

    // Default to the tool's cursor
    commands
        .entity(window_entity)
        .insert(current_tool.tool.cursor_icon());
}
