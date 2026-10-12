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

/// Viewport handle sizes in world units at the editor camera's zoom, so they look and
/// respond the same on screen at any zoom. Drawing and hit testing both use these.
struct ViewportHandles {
    handle: f32,
    tab_size: Vec2,
    tab_offset: f32,
}

impl ViewportHandles {
    fn at_zoom(camera_scale: f32) -> Self {
        Self {
            handle: HANDLE_SIZE * camera_scale,
            tab_size: Vec2::new(MOVE_HANDLE_WIDTH, MOVE_HANDLE_HEIGHT) * camera_scale,
            tab_offset: MOVE_HANDLE_OFFSET * camera_scale,
        }
    }

    /// Center of the move tab above a viewport with half extent `half`
    fn tab_center(&self, center: Vec2, half: Vec2) -> Vec2 {
        center + Vec2::new(0.0, half.y + self.tab_offset)
    }
}

/// Corner and edge handles of a box with half extent `half` around `center`
fn handle_positions(center: Vec2, half: Vec2) -> [(Vec2, ViewportDragMode); 8] {
    [
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
        (center + Vec2::new(0.0, -half.y), ViewportDragMode::ResizeS),
        (center + Vec2::new(0.0, half.y), ViewportDragMode::ResizeN),
        (center + Vec2::new(-half.x, 0.0), ViewportDragMode::ResizeW),
        (center + Vec2::new(half.x, 0.0), ViewportDragMode::ResizeE),
    ]
}

/// The editor camera's zoom (world units per screen pixel)
fn editor_camera_scale(projection: &Projection) -> f32 {
    match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    }
}

/// Draw the viewport indicator rectangle with handles.
/// The box is the world area the player camera shows: at a quarter turn its width and
/// height are the viewport's swapped. The arrow shows which way is "up" on the player
/// display.
pub fn draw_viewport_indicator(
    mut gizmos: Gizmos<ViewportGizmoGroup>,
    session_state: Res<LiveSessionState>,
    player_viewport: Res<PlayerViewport>,
    camera_query: Query<&Projection, With<EditorCamera>>,
) {
    if !session_state.is_active {
        return;
    }
    let camera_scale = camera_query
        .single()
        .map(editor_camera_scale)
        .unwrap_or(1.0);
    let handles = ViewportHandles::at_zoom(camera_scale);

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
        let (min, max) = player_viewport.world_bounds();
        gizmos.rect_2d(
            Isometry2d::from_translation((min + max) / 2.0),
            max - min,
            outline_color.with_alpha(0.35),
        );
    }

    let center = session_state.viewport_center;
    let size = session_state.world_extent();
    let rotation = session_state.rotation_radians();

    // Main outline: the world area players see
    gizmos.rect_2d(Isometry2d::from_translation(center), size, outline_color);

    // A second, slightly larger outline for visibility
    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        size + Vec2::splat(2.0 * camera_scale),
        outline_color.with_alpha(0.5),
    );

    let half = size / 2.0;

    // Corner handles, then smaller edge handles
    for (i, (position, _)) in handle_positions(center, half).into_iter().enumerate() {
        let side = if i < 4 {
            handles.handle
        } else {
            handles.handle * 0.75
        };
        gizmos.rect_2d(
            Isometry2d::from_translation(position),
            Vec2::splat(side),
            outline_color,
        );
    }

    // Move handle: a tab above the top edge, with grip lines
    let tab_center = handles.tab_center(center, half);
    gizmos.rect_2d(
        Isometry2d::from_translation(tab_center),
        handles.tab_size,
        handle_color,
    );
    let grip_half_w = handles.tab_size.x / 2.0 - 8.0 * camera_scale;
    for i in [-4.0, 0.0, 4.0] {
        let y = tab_center.y + i * camera_scale;
        gizmos.line_2d(
            Vec2::new(tab_center.x - grip_half_w, y),
            Vec2::new(tab_center.x + grip_half_w, y),
            handle_color,
        );
    }

    // Connector line from viewport to move handle
    let connector_top = tab_center - Vec2::new(0.0, handles.tab_size.y / 2.0);
    let connector_bottom = center + Vec2::new(0.0, half.y);
    gizmos.line_2d(connector_bottom, connector_top, outline_color);

    // Fill lines for visibility
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
    let half = session_state.world_extent() / 2.0;
    let handles = ViewportHandles::at_zoom(camera_scale);
    let hit_radius = handles.handle * 1.5;

    // Move tab first (highest priority), then corners, then edges
    let to_tab = world_pos - handles.tab_center(center, half);
    if to_tab.x.abs() < handles.tab_size.x / 2.0 && to_tab.y.abs() < handles.tab_size.y / 2.0 {
        return ViewportDragMode::Move;
    }
    handle_positions(center, half)
        .into_iter()
        .find(|(position, _)| position.distance(world_pos) < hit_radius)
        .map(|(_, mode)| mode)
        // Interior is no longer selectable - only handles work
        .unwrap_or(ViewportDragMode::None)
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

    // End a drag on release wherever the mouse is: over a panel or outside the window
    // too, or the next click on the map would carry on the old drag
    if mouse_button.just_released(MouseButton::Left) {
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

    let camera_scale = editor_camera_scale(projection);

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

    // Handle ongoing drag. Resizing works on the world area players see (the box drawn),
    // growing both sides around the center, and keeps the monitor's shape.
    if drag_state.mode != ViewportDragMode::None && mouse_button.pressed(MouseButton::Left) {
        let delta = world_pos - drag_state.drag_start_world;
        let original = if session_state.is_quarter_turned() {
            drag_state.original_size.yx()
        } else {
            drag_state.original_size
        };

        match drag_state.mode {
            ViewportDragMode::Move => {
                session_state.viewport_center = drag_state.original_center + delta;
            }
            // Corners follow the horizontal movement
            ViewportDragMode::ResizeE | ViewportDragMode::ResizeNE | ViewportDragMode::ResizeSE => {
                session_state.set_world_width(original.x + delta.x * 2.0);
            }
            ViewportDragMode::ResizeW | ViewportDragMode::ResizeNW | ViewportDragMode::ResizeSW => {
                session_state.set_world_width(original.x - delta.x * 2.0);
            }
            ViewportDragMode::ResizeN => {
                session_state.set_world_height(original.y + delta.y * 2.0);
            }
            ViewportDragMode::ResizeS => {
                session_state.set_world_height(original.y - delta.y * 2.0);
            }
            ViewportDragMode::Rotate | ViewportDragMode::None => {
                // Rotate is not used for viewport, only for selection
            }
        }
    }
}
