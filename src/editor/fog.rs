//! Fog of War tool for revealing/hiding map areas.
//!
//! The Fog of War layer allows the GM to control what areas of the map players can see.
//! - In the editor: fog appears as semi-transparent grey overlay
//! - In the player view: fog appears as opaque black, completely hiding content beneath
//!
//! ## Tool Behavior
//!
//! - Default (click+drag): Circular brush erases fog from cells within brush radius
//! - Shift+click: Grid-aligned single-cell reveal
//!
//! ## Rendering
//!
//! Uses two gizmo groups:
//! - [`FogEditorGizmoGroup`]: Semi-transparent grey for editor view (RenderLayers::layer(1))
//! - [`FogPlayerGizmoGroup`]: Opaque black for player view (RenderLayers::layer(2))

use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use super::camera::EditorCamera;
use super::params::{is_cursor_over_ui, CameraParams};
use crate::map::{cell_to_world, cells_in_radius, FogOfWarData, Layer, MapData, MapDirtyState};
use crate::session::LiveSessionState;
use crate::theme;

// ============================================================================
// Gizmo Configuration
// ============================================================================

/// Gizmo group for fog rendering in editor view (semi-transparent grey)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FogEditorGizmoGroup;

/// Gizmo group for fog rendering in player view (opaque black)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FogPlayerGizmoGroup;

/// Configure the fog gizmo groups for their respective render layers
pub fn configure_fog_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    // Editor fog: layer 1 (editor-only)
    let (editor_config, _) = config_store.config_mut::<FogEditorGizmoGroup>();
    editor_config.render_layers = RenderLayers::layer(1);

    // Player fog: layer 2 (player-only, not visible in editor)
    // Use wider lines for better coverage
    let (player_config, _) = config_store.config_mut::<FogPlayerGizmoGroup>();
    player_config.render_layers = RenderLayers::layer(2);
    player_config.line.width = 4.0;
}

// ============================================================================
// Layer Helpers
// ============================================================================

/// Check if the FogOfWar layer is visible in editor
pub fn is_fog_layer_visible(map_data: &MapData) -> bool {
    map_data
        .layers
        .iter()
        .find(|ld| ld.layer_type == Layer::FogOfWar)
        .map(|ld| ld.visible)
        .unwrap_or(true)
}

/// Check if the FogOfWar layer is locked
pub fn is_fog_layer_locked(map_data: &MapData) -> bool {
    map_data
        .layers
        .iter()
        .find(|ld| ld.layer_type == Layer::FogOfWar)
        .map(|ld| ld.locked)
        .unwrap_or(false)
}

// ============================================================================
// Fog State
// ============================================================================

/// Resource for fog tool state
#[derive(Resource)]
pub struct FogState {
    /// Brush radius in grid cells (e.g., 2.0 means 2 grid cells radius)
    pub brush_size: f32,
    /// Whether currently erasing fog
    pub is_erasing: bool,
    /// Editor fog opacity (0.0 = invisible, 1.0 = fully opaque)
    pub editor_opacity: f32,
}

impl Default for FogState {
    fn default() -> Self {
        Self {
            brush_size: 2.0,
            is_erasing: false,
            editor_opacity: 0.6,
        }
    }
}

// ============================================================================
// Fog Tool Systems
// ============================================================================

/// Handle fog tool input (revealing cells by erasing fog)
#[allow(clippy::too_many_arguments)]
pub fn handle_fog(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut fog_data: ResMut<FogOfWarData>,
    mut fog_state: ResMut<FogState>,
    map_data: Res<MapData>,
    mut dirty_state: ResMut<MapDirtyState>,
    camera_params: CameraParams,
    mut contexts: EguiContexts,
) {
    // Don't process if layer is locked
    if is_fog_layer_locked(&map_data) {
        return;
    }

    // Don't process if cursor is over UI
    if is_cursor_over_ui(&mut contexts) {
        return;
    }

    let Some(world_pos) = camera_params.cursor_world_pos() else {
        return;
    };

    let grid_size = map_data.grid_size;
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    // Handle mouse input
    if mouse_button.just_pressed(MouseButton::Left) {
        fog_state.is_erasing = true;
    }

    if mouse_button.just_released(MouseButton::Left) {
        fog_state.is_erasing = false;
    }

    // Reveal cells while mouse is held (erase fog)
    if mouse_button.pressed(MouseButton::Left) && fog_state.is_erasing {
        let mut changed = false;

        if shift_held {
            // Grid-aligned mode: reveal single cell under cursor
            let cell = crate::map::world_to_cell(world_pos, grid_size);
            if !fog_data.is_cell_revealed(cell) {
                fog_data.reveal_cell(cell);
                changed = true;
            }
        } else {
            // Circular brush mode: reveal all cells within brush radius
            let brush_radius = fog_state.brush_size * grid_size;
            let cells = cells_in_radius(world_pos, brush_radius, grid_size);
            for cell in cells {
                if !fog_data.is_cell_revealed(cell) {
                    fog_data.reveal_cell(cell);
                    changed = true;
                }
            }
        }

        if changed {
            dirty_state.is_dirty = true;
        }
    }
}

// ============================================================================
// Fog Rendering - Editor View
// ============================================================================

/// Render fog in editor view as filled semi-transparent dark rectangles
///
/// Iterates over all cells in the viewport and renders fog for cells
/// that are NOT in the revealed_cells set. Uses horizontal line fills
/// to create a visible shading effect.
pub fn render_fog_editor(
    mut gizmos: Gizmos<FogEditorGizmoGroup>,
    fog_data: Res<FogOfWarData>,
    fog_state: Res<FogState>,
    map_data: Res<MapData>,
    camera_query: Query<(&Camera, &GlobalTransform, &Projection), With<EditorCamera>>,
) {
    if !is_fog_layer_visible(&map_data) {
        return;
    }

    let grid_size = map_data.grid_size;
    // Use theme fog color with configurable opacity
    let base = theme::FOG_EDITOR_BASE.to_srgba();
    let fog_color = Color::srgba(base.red, base.green, base.blue, fog_state.editor_opacity);

    // Get camera viewport bounds
    let (min_cell, max_cell) = if let Ok((camera, transform, projection)) = camera_query.single() {
        get_viewport_cell_bounds(camera, transform, projection, grid_size)
    } else {
        // Fallback bounds
        ((-50, -50), (50, 50))
    };

    // Line spacing for fill effect - denser lines = more solid appearance
    let line_spacing = 4.0;
    let half_grid = grid_size / 2.0;

    // Render filled fog for all cells in viewport that are NOT revealed
    for x in min_cell.0..=max_cell.0 {
        for y in min_cell.1..=max_cell.1 {
            let cell = (x, y);
            if !fog_data.is_cell_revealed(cell) {
                let center = cell_to_world(cell, grid_size);

                // Draw filled rectangle using horizontal lines
                let mut line_y = -half_grid;
                while line_y <= half_grid {
                    let start = center + Vec2::new(-half_grid, line_y);
                    let end = center + Vec2::new(half_grid, line_y);
                    gizmos.line_2d(start, end, fog_color);
                    line_y += line_spacing;
                }

                // Also draw outline for cleaner edges
                gizmos.rect_2d(
                    Isometry2d::from_translation(center),
                    Vec2::splat(grid_size),
                    fog_color,
                );
            }
        }
    }
}

/// Render brush preview when fog tool is active
pub fn render_fog_brush_preview(
    mut gizmos: Gizmos<FogEditorGizmoGroup>,
    fog_state: Res<FogState>,
    map_data: Res<MapData>,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_params: CameraParams,
) {
    let Some(world_pos) = camera_params.cursor_world_pos() else {
        return;
    };

    let grid_size = map_data.grid_size;
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if shift_held {
        // Grid-aligned mode: show single cell highlight
        let cell = crate::map::world_to_cell(world_pos, grid_size);
        let center = cell_to_world(cell, grid_size);
        gizmos.rect_2d(
            Isometry2d::from_translation(center),
            Vec2::splat(grid_size),
            theme::FOG_BRUSH_CELL_HIGHLIGHT,
        );
    } else {
        // Circular brush mode: show brush circle
        let brush_radius = fog_state.brush_size * grid_size;
        gizmos.circle_2d(
            Isometry2d::from_translation(world_pos),
            brush_radius,
            theme::FOG_BRUSH_CIRCLE,
        );
    }
}

// ============================================================================
// Fog Rendering - Player View
// ============================================================================

/// Render fog in player view as filled opaque black rectangles
///
/// Iterates over all cells in the player viewport and renders fog for cells
/// that are NOT in the revealed_cells set. Uses dense line fills for
/// complete coverage.
pub fn render_fog_player(
    mut gizmos: Gizmos<FogPlayerGizmoGroup>,
    fog_data: Res<FogOfWarData>,
    map_data: Res<MapData>,
    session_state: Res<LiveSessionState>,
) {
    // Only render player fog when session is active and fog layer is visible
    if !session_state.is_active {
        return;
    }

    // Check if fog layer is enabled
    if !is_fog_layer_visible(&map_data) {
        return;
    }

    let grid_size = map_data.grid_size;
    let fog_color = theme::FOG_PLAYER;

    // Calculate rotation-aware viewport bounds
    // When viewport is rotated, we need the axis-aligned bounding box that
    // contains all four corners of the rotated rectangle
    let (min_cell, max_cell) =
        get_rotated_viewport_cell_bounds(&session_state, grid_size);

    // Dense line spacing for player view - want complete black coverage
    // Spacing of 1.0 with line_width of 4.0 ensures overlap at typical zoom levels
    let line_spacing = 1.0;
    let half_grid = grid_size / 2.0;

    // Render filled fog for all cells in viewport that are NOT revealed
    for x in min_cell.0..=max_cell.0 {
        for y in min_cell.1..=max_cell.1 {
            let cell = (x, y);
            if !fog_data.is_cell_revealed(cell) {
                let center = cell_to_world(cell, grid_size);

                // Draw filled rectangle using horizontal lines (dense for player)
                let mut line_y = -half_grid;
                while line_y <= half_grid {
                    let start = center + Vec2::new(-half_grid, line_y);
                    let end = center + Vec2::new(half_grid, line_y);
                    gizmos.line_2d(start, end, fog_color);
                    line_y += line_spacing;
                }
            }
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the cell bounds for the player viewport, accounting for rotation.
///
/// When the viewport is rotated, we need to calculate the axis-aligned bounding
/// box (AABB) that contains all four corners of the rotated rectangle.
fn get_rotated_viewport_cell_bounds(
    session_state: &LiveSessionState,
    grid_size: f32,
) -> ((i32, i32), (i32, i32)) {
    let center = session_state.viewport_center;
    let size = session_state.viewport_size; // Use raw size, not effective
    let half_w = size.x / 2.0;
    let half_h = size.y / 2.0;
    let rotation = session_state.rotation_radians();

    // Calculate the four corners relative to center (before rotation)
    let corners_local = [
        Vec2::new(-half_w, -half_h), // bottom-left
        Vec2::new(half_w, -half_h),  // bottom-right
        Vec2::new(half_w, half_h),   // top-right
        Vec2::new(-half_w, half_h),  // top-left
    ];

    // Rotate each corner and find the AABB
    let cos_r = rotation.cos();
    let sin_r = rotation.sin();

    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for corner in corners_local {
        // Apply rotation: x' = x*cos - y*sin, y' = x*sin + y*cos
        let rotated = Vec2::new(
            corner.x * cos_r - corner.y * sin_r,
            corner.x * sin_r + corner.y * cos_r,
        );
        let world_pos = center + rotated;

        min_x = min_x.min(world_pos.x);
        max_x = max_x.max(world_pos.x);
        min_y = min_y.min(world_pos.y);
        max_y = max_y.max(world_pos.y);
    }

    // Add padding for cells partially visible at edges
    let padding = grid_size * 2.0;
    let min_world = Vec2::new(min_x - padding, min_y - padding);
    let max_world = Vec2::new(max_x + padding, max_y + padding);

    let min_cell = crate::map::world_to_cell(min_world, grid_size);
    let max_cell = crate::map::world_to_cell(max_world, grid_size);

    (min_cell, max_cell)
}

/// Get the cell bounds visible in the camera viewport
fn get_viewport_cell_bounds(
    _camera: &Camera,
    transform: &GlobalTransform,
    projection: &Projection,
    grid_size: f32,
) -> ((i32, i32), (i32, i32)) {
    // Get viewport size from projection
    let viewport_size = match projection {
        Projection::Orthographic(ortho) => Vec2::new(ortho.area.width(), ortho.area.height()),
        _ => Vec2::new(1920.0, 1080.0), // Fallback
    };

    let camera_pos = transform.translation().truncate();
    let half_size = viewport_size / 2.0;

    // Add some padding for cells partially visible at edges
    let padding = grid_size * 2.0;
    let min_world = camera_pos - half_size - Vec2::splat(padding);
    let max_world = camera_pos + half_size + Vec2::splat(padding);

    let min_cell = crate::map::world_to_cell(min_world, grid_size);
    let max_cell = crate::map::world_to_cell(max_world, grid_size);

    (min_cell, max_cell)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fog_state_default() {
        let state = FogState::default();
        assert_eq!(state.brush_size, 2.0);
        assert!(!state.is_erasing);
        assert!((state.editor_opacity - 0.6).abs() < 0.001);
    }
}
