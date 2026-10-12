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
//! The fog itself is a texture per view (see `fog_overlay`); the brush preview is drawn
//! with [`FogEditorGizmoGroup`], editor-only.

use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use super::params::{CameraParams, is_cursor_over_ui};
use crate::input::shift_held;
use crate::map::{
    FogOfWarData, Layer, MapData, MapDirtyState, cell_to_world, cells_in_radius, render_layer,
};
use crate::theme;

// ============================================================================
// Gizmo Configuration
// ============================================================================

/// Gizmo group for the fog brush preview (editor-only)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FogEditorGizmoGroup;

/// Configure the fog brush gizmos for the editor-only render layer
pub fn configure_fog_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (editor_config, _) = config_store.config_mut::<FogEditorGizmoGroup>();
    editor_config.render_layers = RenderLayers::layer(render_layer::EDITOR);
}

// ============================================================================
// Layer Helpers
// ============================================================================

/// Check if the FogOfWar layer is visible in editor
pub fn is_fog_layer_visible(map_data: &MapData) -> bool {
    map_data.is_layer_visible(Layer::FogOfWar)
}

/// Check if the FogOfWar layer is locked
pub fn is_fog_layer_locked(map_data: &MapData) -> bool {
    map_data.is_layer_locked(Layer::FogOfWar)
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
    /// Show the fog in the editor. Only the GM's view: players' fog is the Fog of War
    /// layer's visibility ("Enable Fog of War").
    pub show_in_editor: bool,
}

impl Default for FogState {
    fn default() -> Self {
        Self {
            brush_size: 2.0,
            is_erasing: false,
            editor_opacity: 0.6,
            show_in_editor: true,
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
    // End the stroke on release wherever the mouse is. Released over a panel, a stale
    // stroke would otherwise reveal fog when a later drag from a panel crossed the map.
    if mouse_button.just_released(MouseButton::Left) {
        fog_state.is_erasing = false;
    }

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
    let shift_held = shift_held(&keyboard);

    if mouse_button.just_pressed(MouseButton::Left) {
        fog_state.is_erasing = true;
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
            dirty_state.mark_dirty();
        }
    }
}

// ============================================================================
// Brush Preview (the fog itself is drawn by fog_overlay)
// ============================================================================

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
    let shift_held = shift_held(&keyboard);

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
