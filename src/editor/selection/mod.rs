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

/// Snap increment for rotation when holding Shift (in degrees)
pub(crate) const ROTATION_SNAP_INCREMENT: f32 = 15.0;

/// Sizes of the selection handles, in world units at the current zoom, so they look and
/// respond the same on screen at any zoom. Drawing and hit testing both use these.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HandleMetrics {
    /// Half the side of a corner handle's square
    pub corner: f32,
    /// Half the side of an edge handle's square
    pub edge: f32,
    /// How far the rotation handle sits above the item's top edge
    pub rotation_offset: f32,
    /// Radius of the rotation handle's circle
    pub rotation_radius: f32,
    /// How close the cursor has to be to a handle to grab it
    pub hit_radius: f32,
}

impl HandleMetrics {
    /// Handle sizes for an editor camera at `camera_scale` (world units per screen pixel).
    pub fn at_zoom(camera_scale: f32) -> Self {
        Self {
            corner: 4.0 * camera_scale,
            edge: 3.0 * camera_scale,
            rotation_offset: 25.0 * camera_scale,
            rotation_radius: 6.0 * camera_scale,
            hit_radius: 12.0 * camera_scale,
        }
    }

    /// An item this small on screen gets no edge handles, so clicking it moves it
    /// instead of grabbing a handle that covers its middle.
    pub fn shows_edge_handles(&self, half_size: Vec2) -> bool {
        half_size.min_element() > self.hit_radius * 2.0
    }

    /// An item this small on screen gets no corner handles either.
    pub fn shows_corner_handles(&self, half_size: Vec2) -> bool {
        half_size.min_element() > self.hit_radius
    }
}

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
