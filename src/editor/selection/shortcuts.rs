//! Keyboard shortcuts for selection operations.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map::{MapData, Selected};

use super::hit_detection::get_sprite_half_size;

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

    // G = fit to grid (but not Shift+G, which is center to grid)
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if !keyboard.just_pressed(KeyCode::KeyG) || shift_held {
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

/// Center selected items to the nearest grid cell center when Shift+G is pressed
pub fn handle_center_to_grid(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected_query: Query<&mut Transform, With<Selected>>,
    map_data: Res<MapData>,
    mut contexts: EguiContexts,
) {
    // Don't trigger if typing in UI
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    // Shift+G = center to grid
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if !keyboard.just_pressed(KeyCode::KeyG) || !shift_held {
        return;
    }

    let grid_size = map_data.grid_size;
    let half = grid_size / 2.0;

    for mut transform in selected_query.iter_mut() {
        let pos = transform.translation.truncate();
        // Snap to nearest grid cell center
        let snapped = Vec2::new(
            (pos.x / grid_size).floor() * grid_size + half,
            (pos.y / grid_size).floor() * grid_size + half,
        );
        transform.translation.x = snapped.x;
        transform.translation.y = snapped.y;
    }
}

/// Restore selected items to their original aspect ratio when A is pressed
/// Uses the larger of the two scale values to preserve the largest dimension
pub fn handle_restore_aspect_ratio(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected_query: Query<&mut Transform, With<Selected>>,
    mut contexts: EguiContexts,
) {
    // Don't trigger if typing in UI
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    // A = restore aspect ratio
    if !keyboard.just_pressed(KeyCode::KeyA) {
        return;
    }

    for mut transform in selected_query.iter_mut() {
        // Restore original aspect ratio by making scale uniform
        // Use the larger scale value to preserve the largest dimension
        let uniform_scale = transform.scale.x.abs().max(transform.scale.y.abs());
        transform.scale.x = uniform_scale;
        transform.scale.y = uniform_scale;
    }
}

/// Rotate selected items by 90 degrees when R is pressed (clockwise) or Shift+R (counter-clockwise)
pub fn handle_rotate_90(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected_query: Query<&mut Transform, With<Selected>>,
    mut contexts: EguiContexts,
) {
    // Don't trigger if typing in UI
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    // Rotate 90 degrees: clockwise (negative) or counter-clockwise (positive) with Shift
    let angle = if shift_held { 90.0_f32 } else { -90.0_f32 };
    let rotation_delta = Quat::from_rotation_z(angle.to_radians());

    for mut transform in selected_query.iter_mut() {
        transform.rotation *= rotation_delta;
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

/// Clear selection when Escape is pressed
pub fn handle_escape_clear_selection(
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

    if keyboard.just_pressed(KeyCode::Escape) {
        for entity in selected_query.iter() {
            commands.entity(entity).remove::<Selected>();
        }
    }
}
