//! Selection tool module for the editor.
//!
//! This module handles all selection-related functionality including:
//! - Click and box selection
//! - Drag, resize, and rotate operations
//! - Selection gizmo rendering
//! - Keyboard shortcuts for selected items

mod box_select;
mod cursor;
mod drag;
mod gizmos;
mod handle;
mod hit_detection;
mod shortcuts;

use bevy::prelude::*;
use bevy::window::{CursorIcon, SystemCursorIcon};

// Re-export public items
pub use box_select::handle_box_select;
pub use cursor::update_selection_cursor;
pub use drag::handle_drag;
pub use gizmos::{draw_box_select_rect, draw_selection_indicators};
pub use handle::handle_selection;
// hit_detection items are used internally by submodules but not re-exported
pub use shortcuts::{
    handle_center_to_grid, handle_deletion, handle_escape_clear_selection, handle_fit_to_grid,
    handle_restore_aspect_ratio, handle_rotate_90,
};

/// Distance above selection bounds for the rotation handle (in world units)
pub(crate) const ROTATION_HANDLE_OFFSET: f32 = 25.0;
/// Radius of the rotation handle circle (in world units)
pub(crate) const ROTATION_HANDLE_RADIUS: f32 = 6.0;
/// Snap increment for rotation when holding Shift (in degrees)
pub(crate) const ROTATION_SNAP_INCREMENT: f32 = 15.0;
/// Handle size for resize handles (in world units, will be scaled by camera)
pub(crate) const HANDLE_SIZE: f32 = 8.0;

/// Information about an annotation's original state when dragging started
#[derive(Clone)]
pub enum AnnotationDragData {
    Path { original_points: Vec<Vec2> },
    Line { original_start: Vec2, original_end: Vec2 },
    Text { original_position: Vec2 },
}

/// Drag mode for selection interaction (move, resize, or rotate)
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectionDragMode {
    #[default]
    None,
    Move,
    Rotate,
    ResizeN,
    ResizeS,
    ResizeE,
    ResizeW,
    ResizeNE,
    ResizeNW,
    ResizeSE,
    ResizeSW,
}

impl SelectionDragMode {
    /// Get the appropriate cursor icon for this drag mode
    pub fn cursor_icon(&self) -> Option<CursorIcon> {
        match self {
            SelectionDragMode::None => None,
            SelectionDragMode::Move => Some(CursorIcon::System(SystemCursorIcon::Move)),
            SelectionDragMode::Rotate => Some(CursorIcon::System(SystemCursorIcon::Grab)),
            SelectionDragMode::ResizeN | SelectionDragMode::ResizeS => {
                Some(CursorIcon::System(SystemCursorIcon::NsResize))
            }
            SelectionDragMode::ResizeE | SelectionDragMode::ResizeW => {
                Some(CursorIcon::System(SystemCursorIcon::EwResize))
            }
            SelectionDragMode::ResizeNE | SelectionDragMode::ResizeSW => {
                Some(CursorIcon::System(SystemCursorIcon::NeswResize))
            }
            SelectionDragMode::ResizeNW | SelectionDragMode::ResizeSE => {
                Some(CursorIcon::System(SystemCursorIcon::NwseResize))
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct DragState {
    pub is_dragging: bool,
    pub drag_start_world: Vec2,
    /// The current drag mode (move, resize, or rotate)
    pub mode: SelectionDragMode,
    /// Original selection bounds when resize started (min, max)
    pub original_bounds: Option<(Vec2, Vec2)>,
    /// Maps entity to its starting position when drag began (for PlacedItems)
    pub entity_start_positions: Vec<(Entity, Vec2)>,
    /// Maps entity to its original scale when drag began (for resizing)
    pub entity_start_scales: Vec<(Entity, Vec3)>,
    /// Maps entity to its original rotation when drag began (for rotating)
    pub entity_start_rotations: Vec<(Entity, Quat)>,
    /// Maps entity to its original half-size (sprite size * scale) for rotation-aware resizing
    pub entity_start_half_sizes: Vec<(Entity, Vec2)>,
    /// The starting angle (radians) from selection center to cursor when rotation began
    pub rotation_start_angle: Option<f32>,
    /// Maps entity to its annotation drag data when drag began
    pub annotation_drag_data: Vec<(Entity, AnnotationDragData)>,
}

#[derive(Resource, Default)]
pub struct BoxSelectState {
    pub is_selecting: bool,
    pub start_world: Vec2,
    pub current_world: Vec2,
}
