use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::editor::EditorCamera;

use super::state::{LiveSessionState, ViewportDragMode, ViewportDragState};

const HANDLE_SIZE: f32 = 8.0;
const VIEWPORT_COLOR: Color = Color::srgba(1.0, 0.7, 0.2, 0.9);
const VIEWPORT_FILL_COLOR: Color = Color::srgba(1.0, 0.7, 0.2, 0.1);

/// Custom gizmo group for viewport indicator (editor-only rendering)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct ViewportGizmoGroup;

/// Configure the viewport gizmo group to only render to editor camera
pub fn configure_viewport_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<ViewportGizmoGroup>();
    // Only render to layer 1 (editor-only)
    config.render_layers = RenderLayers::layer(1);
}

/// Rotate a point around a center by the given angle in radians
fn rotate_point(point: Vec2, center: Vec2, angle: f32) -> Vec2 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let translated = point - center;
    Vec2::new(
        translated.x * cos_a - translated.y * sin_a,
        translated.x * sin_a + translated.y * cos_a,
    ) + center
}

/// Draw the viewport indicator rectangle with handles
pub fn draw_viewport_indicator(
    mut gizmos: Gizmos<ViewportGizmoGroup>,
    session_state: Res<LiveSessionState>,
) {
    if !session_state.is_active {
        return;
    }

    let center = session_state.viewport_center;
    let size = session_state.viewport_size;
    let rotation = session_state.rotation_radians();

    // Draw main rectangle outline with rotation
    gizmos.rect_2d(
        Isometry2d::new(center, Rot2::radians(rotation)),
        size,
        VIEWPORT_COLOR,
    );

    // Draw a second slightly offset rectangle for visibility
    gizmos.rect_2d(
        Isometry2d::new(center, Rot2::radians(rotation)),
        size + Vec2::splat(2.0),
        VIEWPORT_COLOR.with_alpha(0.5),
    );

    // Calculate unrotated corner positions relative to center
    let half = size / 2.0;
    let corners_local = [
        Vec2::new(-half.x, -half.y), // SW
        Vec2::new(half.x, -half.y),  // SE
        Vec2::new(half.x, half.y),   // NE
        Vec2::new(-half.x, half.y),  // NW
    ];

    // Draw corner handles (rotated)
    for corner_local in corners_local {
        let corner_world = rotate_point(center + corner_local, center, rotation);
        gizmos.rect_2d(
            Isometry2d::from_translation(corner_world),
            Vec2::splat(HANDLE_SIZE),
            VIEWPORT_COLOR,
        );
    }

    // Calculate unrotated edge midpoints relative to center
    let edges_local = [
        Vec2::new(0.0, -half.y), // S
        Vec2::new(0.0, half.y),  // N
        Vec2::new(-half.x, 0.0), // W
        Vec2::new(half.x, 0.0),  // E
    ];

    // Draw edge handles (rotated)
    for edge_local in edges_local {
        let edge_world = rotate_point(center + edge_local, center, rotation);
        gizmos.rect_2d(
            Isometry2d::from_translation(edge_world),
            Vec2::splat(HANDLE_SIZE * 0.75),
            VIEWPORT_COLOR,
        );
    }

    // Draw rotation indicator (small arrow or line at top)
    let top_center = rotate_point(center + Vec2::new(0.0, half.y + 20.0), center, rotation);
    let arrow_left = rotate_point(center + Vec2::new(-10.0, half.y + 10.0), center, rotation);
    let arrow_right = rotate_point(center + Vec2::new(10.0, half.y + 10.0), center, rotation);
    let arrow_base = rotate_point(center + Vec2::new(0.0, half.y), center, rotation);

    gizmos.line_2d(arrow_base, top_center, VIEWPORT_COLOR);
    gizmos.line_2d(top_center, arrow_left, VIEWPORT_COLOR);
    gizmos.line_2d(top_center, arrow_right, VIEWPORT_COLOR);

    // Draw fill lines for visibility (rotated)
    let step = 40.0;
    let mut y = -half.y + step;
    while y < half.y {
        let line_start = rotate_point(center + Vec2::new(-half.x, y), center, rotation);
        let line_end = rotate_point(center + Vec2::new(half.x, y), center, rotation);
        gizmos.line_2d(line_start, line_end, VIEWPORT_FILL_COLOR);
        y += step;
    }
}

/// Determine which handle (if any) is under the cursor
fn get_handle_at_position(
    world_pos: Vec2,
    session_state: &LiveSessionState,
    camera_scale: f32,
) -> ViewportDragMode {
    let (min, max) = session_state.viewport_bounds();
    let center = session_state.viewport_center;

    // Adjust handle hit area based on camera zoom
    let hit_size = HANDLE_SIZE * camera_scale * 1.5;

    // Check corners first (higher priority)
    let corners = [
        (min, ViewportDragMode::ResizeSW),
        (Vec2::new(max.x, min.y), ViewportDragMode::ResizeSE),
        (max, ViewportDragMode::ResizeNE),
        (Vec2::new(min.x, max.y), ViewportDragMode::ResizeNW),
    ];

    for (corner, mode) in corners {
        if (world_pos - corner).length() < hit_size {
            return mode;
        }
    }

    // Check edge handles
    let edges = [
        (Vec2::new(center.x, min.y), ViewportDragMode::ResizeS),
        (Vec2::new(center.x, max.y), ViewportDragMode::ResizeN),
        (Vec2::new(min.x, center.y), ViewportDragMode::ResizeW),
        (Vec2::new(max.x, center.y), ViewportDragMode::ResizeE),
    ];

    for (edge, mode) in edges {
        if (world_pos - edge).length() < hit_size {
            return mode;
        }
    }

    // Check if inside viewport (for move)
    if world_pos.x >= min.x && world_pos.x <= max.x && world_pos.y >= min.y && world_pos.y <= max.y
    {
        return ViewportDragMode::Move;
    }

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
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.is_pointer_over_area() {
            return;
        }
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
            ViewportDragMode::None => {}
        }
    }
}
