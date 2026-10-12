use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::editor::EditorCamera;
use crate::theme;
use crate::ui::panel_layout::is_pointer_over_ui;

use super::state::{LiveSessionState, PlayerViewport, ViewportDragMode, ViewportDragState};
use crate::map::render_layer;

const HANDLE_SIZE: f32 = 8.0;
const MOVE_HANDLE_WIDTH: f32 = 60.0;
const MOVE_HANDLE_HEIGHT: f32 = 16.0;
const MOVE_HANDLE_OFFSET: f32 = 12.0; // Distance above top edge

/// Custom gizmo group for viewport indicator (editor-only rendering)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct ViewportGizmoGroup;

/// Configure the viewport gizmo group to only render to editor camera
pub fn configure_viewport_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<ViewportGizmoGroup>();
    // Only render to layer 1 (editor-only)
    config.render_layers = RenderLayers::layer(render_layer::EDITOR);
}

/// Draw the viewport indicator rectangle with handles
/// The gizmo shows the actual world-space bounds that the player camera sees.
/// Rotation is indicated by an arrow showing the "up" direction on the player display.
pub fn draw_viewport_indicator(
    mut gizmos: Gizmos<ViewportGizmoGroup>,
    session_state: Res<LiveSessionState>,
    player_viewport: Res<PlayerViewport>,
) {
    if !session_state.is_active {
        return;
    }

    // Ice blue while frozen, as a reminder that players aren't seeing edits
    let (outline_color, fill_color, handle_color) = if session_state.frozen {
        (
            theme::VIEWPORT_FROZEN_OUTLINE,
            theme::VIEWPORT_FROZEN_FILL,
            theme::VIEWPORT_FROZEN_HANDLE,
        )
    } else {
        (
            theme::VIEWPORT_OUTLINE,
            theme::VIEWPORT_FILL,
            theme::VIEWPORT_HANDLE,
        )
    };

    // Where the players are looking, once a frozen view stops following the
    // viewport (see AppConfigData::move_player_view_while_frozen)
    if session_state.frozen && *player_viewport != PlayerViewport::of(&session_state) {
        gizmos.rect_2d(
            Isometry2d::from_translation(player_viewport.center),
            player_viewport.size,
            outline_color.with_alpha(0.35),
        );
    }

    let center = session_state.viewport_center;
    let size = session_state.viewport_size;
    let rotation = session_state.rotation_radians();

    // Draw main rectangle outline (unrotated - shows actual world bounds)
    gizmos.rect_2d(Isometry2d::from_translation(center), size, outline_color);

    // Draw a second slightly offset rectangle for visibility
    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        size + Vec2::splat(2.0),
        outline_color.with_alpha(0.5),
    );

    let half = size / 2.0;

    // Draw corner handles (unrotated)
    let corners = [
        center + Vec2::new(-half.x, -half.y), // SW
        center + Vec2::new(half.x, -half.y),  // SE
        center + Vec2::new(half.x, half.y),   // NE
        center + Vec2::new(-half.x, half.y),  // NW
    ];

    for corner in corners {
        gizmos.rect_2d(
            Isometry2d::from_translation(corner),
            Vec2::splat(HANDLE_SIZE),
            outline_color,
        );
    }

    // Draw edge handles (unrotated)
    let edges = [
        center + Vec2::new(0.0, -half.y), // S
        center + Vec2::new(0.0, half.y),  // N
        center + Vec2::new(-half.x, 0.0), // W
        center + Vec2::new(half.x, 0.0),  // E
    ];

    for edge in edges {
        gizmos.rect_2d(
            Isometry2d::from_translation(edge),
            Vec2::splat(HANDLE_SIZE * 0.75),
            outline_color,
        );
    }

    // Draw move handle (small tab above top edge)
    let move_handle_center = center + Vec2::new(0.0, half.y + MOVE_HANDLE_OFFSET);

    // Draw move handle rectangle
    gizmos.rect_2d(
        Isometry2d::from_translation(move_handle_center),
        Vec2::new(MOVE_HANDLE_WIDTH, MOVE_HANDLE_HEIGHT),
        handle_color,
    );

    // Draw filled appearance with horizontal lines in move handle
    let handle_half_h = MOVE_HANDLE_HEIGHT / 2.0;
    let handle_half_w = MOVE_HANDLE_WIDTH / 2.0;
    for i in [-4.0, 0.0, 4.0] {
        let line_start = center + Vec2::new(-handle_half_w + 8.0, half.y + MOVE_HANDLE_OFFSET + i);
        let line_end = center + Vec2::new(handle_half_w - 8.0, half.y + MOVE_HANDLE_OFFSET + i);
        gizmos.line_2d(line_start, line_end, handle_color);
    }

    // Draw connector line from viewport to move handle
    let connector_top = center + Vec2::new(0.0, half.y + MOVE_HANDLE_OFFSET - handle_half_h);
    let connector_bottom = center + Vec2::new(0.0, half.y);
    gizmos.line_2d(connector_bottom, connector_top, outline_color);

    // Draw fill lines for visibility (unrotated)
    let step = 40.0;
    let mut y = -half.y + step;
    while y < half.y {
        let line_start = center + Vec2::new(-half.x, y);
        let line_end = center + Vec2::new(half.x, y);
        gizmos.line_2d(line_start, line_end, fill_color);
        y += step;
    }

    // Draw rotation indicator arrow showing "up" direction on the player display
    // The arrow points in the direction that will appear as "up" on the player screen
    let arrow_length = half.y.min(half.x) * 0.3;
    let arrow_head_size = arrow_length * 0.3;

    // Arrow direction: rotated "up" vector (what world direction is "up" on the display)
    // When rotation is 0, up on display = +Y in world
    // When rotation is 90, up on display = +X in world (because camera rotates -90)
    let up_dir = Vec2::new(rotation.sin(), rotation.cos());

    let arrow_start = center;
    let arrow_end = center + up_dir * arrow_length;

    // Draw arrow shaft
    gizmos.line_2d(arrow_start, arrow_end, outline_color);

    // Draw arrow head
    let perp = Vec2::new(-up_dir.y, up_dir.x);
    let head_base = arrow_end - up_dir * arrow_head_size;
    gizmos.line_2d(
        arrow_end,
        head_base + perp * arrow_head_size * 0.5,
        outline_color,
    );
    gizmos.line_2d(
        arrow_end,
        head_base - perp * arrow_head_size * 0.5,
        outline_color,
    );
}

/// Determine which handle (if any) is under the cursor
pub fn get_handle_at_position(
    world_pos: Vec2,
    session_state: &LiveSessionState,
    camera_scale: f32,
) -> ViewportDragMode {
    let center = session_state.viewport_center;
    let half = session_state.viewport_size / 2.0;

    // Adjust handle hit area based on camera zoom
    let hit_size = HANDLE_SIZE * camera_scale * 1.5;

    // Check move handle first (highest priority - the tab above the viewport)
    let move_handle_center = center + Vec2::new(0.0, half.y + MOVE_HANDLE_OFFSET);
    let move_handle_hit_width = MOVE_HANDLE_WIDTH * camera_scale * 0.6;
    let move_handle_hit_height = MOVE_HANDLE_HEIGHT * camera_scale * 0.8;

    let to_handle = world_pos - move_handle_center;
    if to_handle.x.abs() < move_handle_hit_width / 2.0
        && to_handle.y.abs() < move_handle_hit_height / 2.0
    {
        return ViewportDragMode::Move;
    }

    // Check corners (higher priority than edges)
    let corners = [
        (
            center + Vec2::new(-half.x, -half.y),
            ViewportDragMode::ResizeSW,
        ),
        (
            center + Vec2::new(half.x, -half.y),
            ViewportDragMode::ResizeSE,
        ),
        (
            center + Vec2::new(half.x, half.y),
            ViewportDragMode::ResizeNE,
        ),
        (
            center + Vec2::new(-half.x, half.y),
            ViewportDragMode::ResizeNW,
        ),
    ];

    for (corner, mode) in corners {
        if (world_pos - corner).length() < hit_size {
            return mode;
        }
    }

    // Check edge handles
    let edges = [
        (center + Vec2::new(0.0, -half.y), ViewportDragMode::ResizeS),
        (center + Vec2::new(0.0, half.y), ViewportDragMode::ResizeN),
        (center + Vec2::new(-half.x, 0.0), ViewportDragMode::ResizeW),
        (center + Vec2::new(half.x, 0.0), ViewportDragMode::ResizeE),
    ];

    for (edge, mode) in edges {
        if (world_pos - edge).length() < hit_size {
            return mode;
        }
    }

    // Interior is no longer selectable - only handles work
    ViewportDragMode::None
}

/// Handle viewport move and resize interactions
pub fn handle_viewport_interaction(
    mouse_button: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform, &Projection), With<EditorCamera>>,
    mut session_state: ResMut<LiveSessionState>,
    mut drag_state: ResMut<ViewportDragState>,
    mut contexts: EguiContexts,
) {
    if !session_state.is_active {
        drag_state.mode = ViewportDragMode::None;
        return;
    }

    // Don't interact if over UI
    if let Ok(ctx) = contexts.ctx_mut()
        && is_pointer_over_ui(ctx)
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

    // Get the camera scale from projection
    let camera_scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };

    // Handle mouse press - start drag
    if mouse_button.just_pressed(MouseButton::Left) {
        let mode = get_handle_at_position(world_pos, &session_state, camera_scale);
        if mode != ViewportDragMode::None {
            drag_state.mode = mode;
            drag_state.drag_start_world = world_pos;
            drag_state.original_center = session_state.viewport_center;
            drag_state.original_size = session_state.viewport_size;
        }
    }

    // Handle mouse release - end drag
    if mouse_button.just_released(MouseButton::Left) {
        drag_state.mode = ViewportDragMode::None;
    }

    // Handle ongoing drag
    if drag_state.mode != ViewportDragMode::None && mouse_button.pressed(MouseButton::Left) {
        let delta = world_pos - drag_state.drag_start_world;
        let aspect_ratio = session_state.monitor_aspect_ratio();

        match drag_state.mode {
            ViewportDragMode::Move => {
                session_state.viewport_center = drag_state.original_center + delta;
            }
            ViewportDragMode::ResizeE => {
                let new_width = (drag_state.original_size.x + delta.x * 2.0).max(100.0);
                session_state.viewport_size.x = new_width;
                session_state.viewport_size.y = new_width / aspect_ratio;
            }
            ViewportDragMode::ResizeW => {
                let new_width = (drag_state.original_size.x - delta.x * 2.0).max(100.0);
                session_state.viewport_size.x = new_width;
                session_state.viewport_size.y = new_width / aspect_ratio;
            }
            ViewportDragMode::ResizeN => {
                let new_height = (drag_state.original_size.y + delta.y * 2.0).max(100.0);
                session_state.viewport_size.y = new_height;
                session_state.viewport_size.x = new_height * aspect_ratio;
            }
            ViewportDragMode::ResizeS => {
                let new_height = (drag_state.original_size.y - delta.y * 2.0).max(100.0);
                session_state.viewport_size.y = new_height;
                session_state.viewport_size.x = new_height * aspect_ratio;
            }
            ViewportDragMode::ResizeNE | ViewportDragMode::ResizeSE => {
                // Use horizontal movement as primary
                let new_width = (drag_state.original_size.x + delta.x * 2.0).max(100.0);
                session_state.viewport_size.x = new_width;
                session_state.viewport_size.y = new_width / aspect_ratio;
            }
            ViewportDragMode::ResizeNW | ViewportDragMode::ResizeSW => {
                // Use horizontal movement as primary
                let new_width = (drag_state.original_size.x - delta.x * 2.0).max(100.0);
                session_state.viewport_size.x = new_width;
                session_state.viewport_size.y = new_width / aspect_ratio;
            }
            ViewportDragMode::Rotate | ViewportDragMode::None => {
                // Rotate is not used for viewport, only for selection
            }
        }
    }
}
