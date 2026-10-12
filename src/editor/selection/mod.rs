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
mod resize;
mod shortcuts;

use bevy::prelude::*;

use crate::common::DragMode;

/// Type alias for selection drag mode (uses common DragMode)
pub type SelectionDragMode = DragMode;

// Re-export public items
pub use box_select::handle_box_select;
pub use cursor::update_selection_cursor;
pub use drag::handle_drag;
pub use gizmos::{
    SelectionGizmoGroup, configure_selection_gizmos, draw_box_select_rect,
    draw_selection_indicators,
};
pub use handle::handle_selection;
// hit_detection items are used internally by submodules but not re-exported
pub use shortcuts::{
    SelectionEdit, apply_selection_edits, deselect_on_locked_or_hidden_layers, handle_deletion,
    handle_escape_clear_selection, selection_edit_shortcuts,
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
    Path {
        original_points: Vec<Vec2>,
    },
    Line {
        original_start: Vec2,
        original_end: Vec2,
    },
    Text {
        original_position: Vec2,
    },
}

/// A placed item as it was when a drag started
#[derive(Clone, Copy, Debug)]
pub struct ItemDragStart {
    pub entity: Entity,
    pub transform: Transform,
    /// Half its size in its own frame: sprite size times scale, always positive
    pub half_size: Vec2,
}

#[derive(Resource, Default)]
pub struct DragState {
    pub is_dragging: bool,
    pub drag_start_world: Vec2,
    /// The current drag mode (move, resize, or rotate)
    pub mode: SelectionDragMode,
    /// Original selection bounds when the drag started (min, max)
    pub original_bounds: Option<(Vec2, Vec2)>,
    /// Placed items being dragged, as they were when the drag started
    pub items: Vec<ItemDragStart>,
    /// For a resize, the item whose handle was grabbed; the others scale like it
    pub resize_reference: Option<Entity>,
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
