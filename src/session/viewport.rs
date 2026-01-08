use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::editor::EditorCamera;

use super::state::{LiveSessionState, ViewportDragMode, ViewportDragState};

const HANDLE_SIZE: f32 = 8.0;
const MOVE_HANDLE_WIDTH: f32 = 60.0;
const MOVE_HANDLE_HEIGHT: f32 = 16.0;
const MOVE_HANDLE_OFFSET: f32 = 12.0; // Distance above top edge
const VIEWPORT_COLOR: Color = Color::srgba(1.0, 0.7, 0.2, 0.9);
const VIEWPORT_FILL_COLOR: Color = Color::srgba(1.0, 0.7, 0.2, 0.1);
const MOVE_HANDLE_COLOR: Color = Color::srgba(1.0, 0.7, 0.2, 1.0);

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

    // Draw move handle (small tab above top edge)
    let move_handle_center =
        rotate_point(center + Vec2::new(0.0, half.y + MOVE_HANDLE_OFFSET), center, rotation);

    // Draw move handle rectangle
    gizmos.rect_2d(
        Isometry2d::new(move_handle_center, Rot2::radians(rotation)),
        Vec2::new(MOVE_HANDLE_WIDTH, MOVE_HANDLE_HEIGHT),
        MOVE_HANDLE_COLOR,
    );

    // Draw filled appearance with horizontal lines
    let handle_half_h = MOVE_HANDLE_HEIGHT / 2.0;
    let handle_half_w = MOVE_HANDLE_WIDTH / 2.0;
    for i in [-4.0, 0.0, 4.0] {
        let line_start = rotate_point(
            center + Vec2::new(-handle_half_w + 8.0, half.y + MOVE_HANDLE_OFFSET + i),
            center,
            rotation,
        );
        let line_end = rotate_point(
            center + Vec2::new(handle_half_w - 8.0, half.y + MOVE_HANDLE_OFFSET + i),
            center,
            rotation,
        );
        gizmos.line_2d(line_start, line_end, MOVE_HANDLE_COLOR);
    }

    // Draw connector line from viewport to move handle
    let connector_top = rotate_point(
        center + Vec2::new(0.0, half.y + MOVE_HANDLE_OFFSET - handle_half_h),
        center,
        rotation,
    );
    let connector_bottom = rotate_point(center + Vec2::new(0.0, half.y), center, rotation);
    gizmos.line_2d(connector_bottom, connector_top, VIEWPORT_COLOR);

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
pub fn get_handle_at_position(
    world_pos: Vec2,
    session_state: &LiveSessionState,
    camera_scale: f32,
) -> ViewportDragMode {
    let (min, max) = session_state.viewport_bounds();
    let center = session_state.viewport_center;
    let half = session_state.viewport_size / 2.0;
    let rotation = session_state.rotation_radians();

    // Adjust handle hit area based on camera zoom
    let hit_size = HANDLE_SIZE * camera_scale * 1.5;

    // Check move handle first (highest priority - the tab above the viewport)
    let move_handle_center =
        rotate_point(center + Vec2::new(0.0, half.y + MOVE_HANDLE_OFFSET), center, rotation);
    let move_handle_hit_width = MOVE_HANDLE_WIDTH * camera_scale * 0.6;
    let move_handle_hit_height = MOVE_HANDLE_HEIGHT * camera_scale * 0.8;

    // Transform world_pos to move handle's local space (accounting for rotation)
    let to_handle = world_pos - move_handle_center;
    let cos_r = (-rotation).cos();
    let sin_r = (-rotation).sin();
    let local_pos = Vec2::new(
        to_handle.x * cos_r - to_handle.y * sin_r,
        to_handle.x * sin_r + to_handle.y * cos_r,
    );

    if local_pos.x.abs() < move_handle_hit_width / 2.0
        && local_pos.y.abs() < move_handle_hit_height / 2.0
    {
        return ViewportDragMode::Move;
    }

    // Check corners (higher priority than edges)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    // rotate_point tests
    #[test]
    fn test_rotate_point_no_rotation() {
        let point = Vec2::new(100.0, 0.0);
        let center = Vec2::ZERO;
        let result = rotate_point(point, center, 0.0);
        assert!((result.x - 100.0).abs() < 0.001);
        assert!(result.y.abs() < 0.001);
    }

    #[test]
    fn test_rotate_point_90_degrees() {
        let point = Vec2::new(100.0, 0.0);
        let center = Vec2::ZERO;
        let result = rotate_point(point, center, PI / 2.0);
        // (100, 0) rotated 90° CCW around origin becomes approximately (0, 100)
        assert!(result.x.abs() < 0.001, "x should be ~0, was {}", result.x);
        assert!((result.y - 100.0).abs() < 0.001, "y should be ~100, was {}", result.y);
    }

    #[test]
    fn test_rotate_point_180_degrees() {
        let point = Vec2::new(100.0, 0.0);
        let center = Vec2::ZERO;
        let result = rotate_point(point, center, PI);
        // (100, 0) rotated 180° around origin becomes (-100, 0)
        assert!((result.x + 100.0).abs() < 0.001, "x should be ~-100, was {}", result.x);
        assert!(result.y.abs() < 0.001, "y should be ~0, was {}", result.y);
    }

    #[test]
    fn test_rotate_point_270_degrees() {
        let point = Vec2::new(100.0, 0.0);
        let center = Vec2::ZERO;
        let result = rotate_point(point, center, 3.0 * PI / 2.0);
        // (100, 0) rotated 270° CCW around origin becomes approximately (0, -100)
        assert!(result.x.abs() < 0.001, "x should be ~0, was {}", result.x);
        assert!((result.y + 100.0).abs() < 0.001, "y should be ~-100, was {}", result.y);
    }

    #[test]
    fn test_rotate_point_full_rotation() {
        let point = Vec2::new(100.0, 50.0);
        let center = Vec2::ZERO;
        let result = rotate_point(point, center, 2.0 * PI);
        // Full rotation should return to original position
        assert!((result.x - 100.0).abs() < 0.001);
        assert!((result.y - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_rotate_point_around_non_origin_center() {
        let point = Vec2::new(200.0, 100.0);
        let center = Vec2::new(100.0, 100.0);
        let result = rotate_point(point, center, PI / 2.0);
        // Point is 100 units to the right of center
        // After 90° CCW rotation, it should be 100 units above center
        assert!((result.x - 100.0).abs() < 0.001, "x should be ~100, was {}", result.x);
        assert!((result.y - 200.0).abs() < 0.001, "y should be ~200, was {}", result.y);
    }

    #[test]
    fn test_rotate_point_at_center() {
        // Point at center shouldn't move regardless of rotation
        let center = Vec2::new(50.0, 50.0);
        let point = center;
        let result = rotate_point(point, center, PI / 3.0);
        assert!((result.x - 50.0).abs() < 0.001);
        assert!((result.y - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_rotate_point_45_degrees() {
        let point = Vec2::new(1.0, 0.0);
        let center = Vec2::ZERO;
        let result = rotate_point(point, center, PI / 4.0);
        // (1, 0) rotated 45° should be approximately (0.707, 0.707)
        let expected = 1.0 / 2.0_f32.sqrt();
        assert!((result.x - expected).abs() < 0.001);
        assert!((result.y - expected).abs() < 0.001);
    }

    #[test]
    fn test_rotate_point_negative_angle() {
        let point = Vec2::new(100.0, 0.0);
        let center = Vec2::ZERO;
        let result = rotate_point(point, center, -PI / 2.0);
        // (100, 0) rotated -90° (clockwise) around origin becomes approximately (0, -100)
        assert!(result.x.abs() < 0.001, "x should be ~0, was {}", result.x);
        assert!((result.y + 100.0).abs() < 0.001, "y should be ~-100, was {}", result.y);
    }

    #[test]
    fn test_rotate_point_preserves_distance() {
        let point = Vec2::new(100.0, 50.0);
        let center = Vec2::new(20.0, 30.0);
        let original_distance = (point - center).length();

        // Test various rotation angles
        for angle in [0.0, PI / 6.0, PI / 4.0, PI / 3.0, PI / 2.0, PI, 3.0 * PI / 2.0] {
            let result = rotate_point(point, center, angle);
            let new_distance = (result - center).length();
            assert!(
                (original_distance - new_distance).abs() < 0.001,
                "Distance should be preserved at angle {}, was {} vs {}",
                angle, new_distance, original_distance
            );
        }
    }
}
