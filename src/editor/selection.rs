use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::map::{MapData, PlacedItem, Selected};

use super::tools::{CurrentTool, EditorTool};
use super::EditorCamera;

#[derive(Resource, Default)]
pub struct DragState {
    pub is_dragging: bool,
    pub drag_start_world: Vec2,
    /// Maps entity to its starting position when drag began
    pub entity_start_positions: Vec<(Entity, Vec2)>,
}

#[derive(Resource, Default)]
pub struct BoxSelectState {
    pub is_selecting: bool,
    pub start_world: Vec2,
    pub current_world: Vec2,
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

pub fn handle_selection(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    items_query: Query<(Entity, &Transform, &Sprite, &PlacedItem)>,
    selected_query: Query<Entity, With<Selected>>,
    mut drag_state: ResMut<DragState>,
    mut box_select_state: ResMut<BoxSelectState>,
    mut contexts: EguiContexts,
    images: Res<Assets<Image>>,
    map_data: Res<MapData>,
) {
    if current_tool.tool != EditorTool::Select {
        return;
    }

    // Don't interact if over UI
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.is_pointer_over_area() {
            return;
        }
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

    let ctrl_held = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

    if mouse_button.just_pressed(MouseButton::Left) {
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

        let clicked_item = items.iter().find(|(_, transform, sprite, _)| {
            point_in_item(world_pos, transform, sprite, &images)
        });

        if let Some(&(entity, transform, _, _)) = clicked_item {
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
                drag_state.is_dragging = true;
                drag_state.drag_start_world = world_pos;
                drag_state.entity_start_positions = selected_query
                    .iter()
                    .filter_map(|e| {
                        items_query.get(e).ok().map(|(entity, t, _, _)| {
                            (entity, t.translation.truncate())
                        })
                    })
                    .collect();
            } else {
                // Clicked on unselected item: clear selection and select this one
                for entity in selected_query.iter() {
                    commands.entity(entity).remove::<Selected>();
                }
                commands.entity(entity).insert(Selected);

                // Start dragging this item
                drag_state.is_dragging = true;
                drag_state.drag_start_world = world_pos;
                drag_state.entity_start_positions = vec![(entity, transform.translation.truncate())];
            }
        } else {
            // Clicked on empty space
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

pub fn handle_box_select(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    items_query: Query<(Entity, &Transform, &Sprite), With<PlacedItem>>,
    selected_query: Query<Entity, With<Selected>>,
    mut box_select_state: ResMut<BoxSelectState>,
    mut contexts: EguiContexts,
    images: Res<Assets<Image>>,
) {
    if current_tool.tool != EditorTool::Select {
        box_select_state.is_selecting = false;
        return;
    }

    if !box_select_state.is_selecting {
        return;
    }

    // Don't update if over UI
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.is_pointer_over_area() {
            return;
        }
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
        let drag_distance = (box_select_state.current_world - box_select_state.start_world).length();
        if drag_distance < 5.0 {
            return;
        }

        let ctrl_held = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

        // If not holding ctrl, clear existing selection
        if !ctrl_held {
            for entity in selected_query.iter() {
                commands.entity(entity).remove::<Selected>();
            }
        }

        // Select all items that overlap with the selection rectangle
        for (entity, transform, sprite) in items_query.iter() {
            if item_overlaps_rect(rect_min, rect_max, transform, sprite, &images) {
                if ctrl_held && selected_query.contains(entity) {
                    // Ctrl + box select: toggle (deselect if already selected)
                    commands.entity(entity).remove::<Selected>();
                } else {
                    commands.entity(entity).insert(Selected);
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
) {
    if current_tool.tool != EditorTool::Select {
        drag_state.is_dragging = false;
        return;
    }

    // Stop dragging on mouse release
    if mouse_button.just_released(MouseButton::Left) {
        drag_state.is_dragging = false;
        return;
    }

    if !drag_state.is_dragging {
        return;
    }

    // Don't drag if over UI
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.is_pointer_over_area() {
            return;
        }
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

    // Shift = snap the offset to grid increments
    if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        drag_offset.x = (drag_offset.x / map_data.grid_size).round() * map_data.grid_size;
        drag_offset.y = (drag_offset.y / map_data.grid_size).round() * map_data.grid_size;
    }

    // Apply offset to each entity, maintaining relative positions
    for (entity, start_pos) in &drag_state.entity_start_positions {
        if let Ok(mut transform) = items_query.get_mut(*entity) {
            let new_pos = *start_pos + drag_offset;
            transform.translation.x = new_pos.x;
            transform.translation.y = new_pos.y;
        }
    }
}

pub fn draw_selection_indicators(
    mut gizmos: Gizmos,
    selected_query: Query<(&Transform, &Sprite), With<Selected>>,
    images: Res<Assets<Image>>,
) {
    let selection_color = Color::srgb(0.2, 0.6, 1.0);

    for (transform, sprite) in selected_query.iter() {
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

        // Draw corner handles
        let handle_size = 4.0;
        let corners = [
            pos + Vec2::new(-scaled_half.x, -scaled_half.y),
            pos + Vec2::new(scaled_half.x, -scaled_half.y),
            pos + Vec2::new(scaled_half.x, scaled_half.y),
            pos + Vec2::new(-scaled_half.x, scaled_half.y),
        ];

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
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.wants_keyboard_input() {
            return;
        }
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
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.wants_keyboard_input() {
            return;
        }
    }

    let should_delete =
        keyboard.just_pressed(KeyCode::Delete) || keyboard.just_pressed(KeyCode::Backspace);

    if should_delete {
        for entity in selected_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}
