//! Cursor icon management for selection tool.

use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use bevy_egui::EguiContexts;

use crate::editor::camera::EditorCamera;
use crate::editor::tools::{CurrentTool, EditorTool};
use crate::map::Selected;

use super::hit_detection::{check_rotation_handle_hit, get_selection_handle_at_position};
use super::{DragState, SelectionDragMode};

/// Update cursor icon based on hover over selection handles
#[allow(clippy::too_many_arguments)]
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
    // Check rotation handle first (it's per-item and accounts for rotation)
    let hover_mode = if check_rotation_handle_hit(
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

    if let Some(cursor) = hover_mode.cursor_icon() {
        commands.entity(window_entity).insert(cursor);
        return;
    }

    // Default to the tool's cursor
    commands
        .entity(window_entity)
        .insert(current_tool.tool.cursor_icon());
}
