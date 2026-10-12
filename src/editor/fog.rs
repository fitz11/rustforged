//! Fog of War tool for revealing/hiding map areas.
//!
//! The Fog of War layer allows the GM to control what areas of the map players can see.
//! - In the editor: fog appears as semi-transparent grey overlay
//! - In the player view: fog appears as opaque black, completely hiding content beneath
//!
//! ## Tool Behavior
//!
//! - Left drag: the circular brush reveals cells
//! - Right drag: the brush covers cells with fog again
//! - Shift+drag (either button): reveal or cover a rectangle of cells; a Shift+click is a
//!   one-cell rectangle
//!
//! Each stroke is one undo step.
//!
//! ## Rendering
//!
//! The fog itself is a texture per view (see `fog_overlay`); the brush preview is drawn
//! with [`FogEditorGizmoGroup`], editor-only.

use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use super::history::{EditorCommand, RecordEditorCommand};
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

/// A Fog tool stroke in progress
#[derive(Debug, Default)]
pub struct FogStroke {
    /// Covering cells (right button) rather than revealing them (left button)
    pub hide: bool,
    /// Shift was held when the stroke began: a rectangle from here to the cursor
    pub rect_start: Option<Vec2>,
    /// Where the cursor last was over the map, for finishing a rectangle released over a
    /// panel
    pub last_pos: Vec2,
    /// Cells the stroke revealed or covered, for its undo step
    pub changed: HashSet<(i32, i32)>,
}

impl FogStroke {
    fn button(&self) -> MouseButton {
        if self.hide {
            MouseButton::Right
        } else {
            MouseButton::Left
        }
    }

    /// Reveals (or covers) `cells`, remembering the ones that changed.
    fn apply(&mut self, cells: impl IntoIterator<Item = (i32, i32)>, fog: &mut FogOfWarData) {
        for cell in cells {
            if fog.is_cell_revealed(cell) == self.hide {
                if self.hide {
                    fog.fog_cell(cell);
                } else {
                    fog.reveal_cell(cell);
                }
                self.changed.insert(cell);
            }
        }
    }

    /// The stroke as one undo step, if it changed anything.
    fn into_command(self) -> Option<EditorCommand> {
        let cells = self.changed.into_iter().collect();
        if self.hide {
            EditorCommand::fog(Vec::new(), cells)
        } else {
            EditorCommand::fog(cells, Vec::new())
        }
    }
}

/// Every cell in the rectangle with corners in the cells under `a` and `b`
pub fn cells_in_rect(a: Vec2, b: Vec2, grid_size: f32) -> impl Iterator<Item = (i32, i32)> {
    let (ax, ay) = crate::map::world_to_cell(a, grid_size);
    let (bx, by) = crate::map::world_to_cell(b, grid_size);
    (ax.min(bx)..=ax.max(bx)).flat_map(move |x| (ay.min(by)..=ay.max(by)).map(move |y| (x, y)))
}

/// Resource for fog tool state
#[derive(Resource)]
pub struct FogState {
    /// Brush radius in grid cells (e.g., 2.0 means 2 grid cells radius)
    pub brush_size: f32,
    /// The stroke in progress, if any
    pub stroke: Option<FogStroke>,
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
            stroke: None,
            editor_opacity: 0.6,
            show_in_editor: true,
        }
    }
}

// ============================================================================
// Fog Tool Systems
// ============================================================================

/// Handle fog tool input: brush or rectangle strokes that reveal (left button) or cover
/// (right button) cells, each recorded as one undo step
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
    mut history: MessageWriter<RecordEditorCommand>,
) {
    let grid_size = map_data.grid_size;

    // End the stroke on release wherever the mouse is (over a panel too): a stale stroke
    // would otherwise reveal fog when a later drag from a panel crossed the map
    if let Some(stroke) = &fog_state.stroke
        && mouse_button.just_released(stroke.button())
        && let Some(mut stroke) = fog_state.stroke.take()
    {
        if let Some(start) = stroke.rect_start {
            let end = stroke.last_pos;
            stroke.apply(
                cells_in_rect(start, end, grid_size),
                fog_data.bypass_change_detection(),
            );
            fog_data.set_changed();
        }
        if !stroke.changed.is_empty() {
            dirty_state.mark_dirty();
        }
        if let Some(command) = stroke.into_command() {
            history.write(RecordEditorCommand { command });
        }
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

    if fog_state.stroke.is_none() {
        let hide = if mouse_button.just_pressed(MouseButton::Left) {
            Some(false)
        } else if mouse_button.just_pressed(MouseButton::Right) {
            Some(true)
        } else {
            None
        };
        if let Some(hide) = hide {
            fog_state.stroke = Some(FogStroke {
                hide,
                rect_start: shift_held(&keyboard).then_some(world_pos),
                last_pos: world_pos,
                changed: HashSet::new(),
            });
        }
    }

    let brush_radius = fog_state.brush_size * grid_size;
    let Some(stroke) = fog_state.stroke.as_mut() else {
        return;
    };
    stroke.last_pos = world_pos;
    // A rectangle is applied when it's released; the brush paints as it goes
    if stroke.rect_start.is_none() && mouse_button.pressed(stroke.button()) {
        let before = stroke.changed.len();
        stroke.apply(
            cells_in_radius(world_pos, brush_radius, grid_size),
            fog_data.bypass_change_detection(),
        );
        if stroke.changed.len() != before {
            fog_data.set_changed();
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
    let rect_start = fog_state.stroke.as_ref().and_then(|s| s.rect_start);

    if let Some(start) = rect_start {
        // Rectangle being dragged: the cells it covers
        let (ax, ay) = crate::map::world_to_cell(start, grid_size);
        let (bx, by) = crate::map::world_to_cell(world_pos, grid_size);
        let min = Vec2::new(ax.min(bx) as f32, ay.min(by) as f32) * grid_size;
        let max = Vec2::new((ax.max(bx) + 1) as f32, (ay.max(by) + 1) as f32) * grid_size;
        gizmos.rect_2d(
            Isometry2d::from_translation((min + max) / 2.0),
            max - min,
            theme::FOG_BRUSH_CELL_HIGHLIGHT,
        );
    } else if shift_held(&keyboard) {
        // About to drag a rectangle: the cell it starts from
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
    fn test_cells_in_rect_covers_both_corners() {
        let cells: Vec<_> =
            cells_in_rect(Vec2::new(75.0, 5.0), Vec2::new(-5.0, 140.0), 70.0).collect();
        assert_eq!(cells.len(), 3 * 3);
        assert!(cells.contains(&(-1, 0)) && cells.contains(&(1, 2)));
    }

    #[test]
    fn test_stroke_records_only_cells_it_changed() {
        let mut fog = FogOfWarData::default();
        fog.reveal_cell((0, 0));

        let mut reveal = FogStroke::default();
        reveal.apply([(0, 0), (1, 0)], &mut fog);
        assert_eq!(
            reveal.into_command(),
            Some(EditorCommand::Fog {
                revealed: vec![(1, 0)],
                hidden: vec![]
            })
        );

        let mut cover = FogStroke {
            hide: true,
            ..default()
        };
        cover.apply([(1, 0), (5, 5)], &mut fog);
        assert!(!fog.is_cell_revealed((1, 0)));
        assert_eq!(
            cover.into_command(),
            Some(EditorCommand::Fog {
                revealed: vec![],
                hidden: vec![(1, 0)]
            })
        );
        assert_eq!(FogStroke::default().into_command(), None);
    }

    #[test]
    fn test_fog_state_default() {
        let state = FogState::default();
        assert_eq!(state.brush_size, 2.0);
        assert!(state.stroke.is_none());
        assert!((state.editor_opacity - 0.6).abs() < 0.001);
    }
}
