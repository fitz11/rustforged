//! Annotation system for drawing on maps (editor-only layer).
//!
//! Annotations allow users to mark up maps with freehand paths, straight lines, and text.
//! These are stored on the Annotation layer (z=350) and are only visible in the editor,
//! not in the player view.
//!
//! ## Module Structure
//!
//! - [`components`] - Entity components (DrawnPath, DrawnLine, TextAnnotation)
//! - [`state`] - State resources (DrawState, LineDrawState, AnnotationSettings)
//! - [`gizmo`] - Custom gizmo group for editor-only rendering
//! - [`hit_testing`] - Hit detection functions for selection
//! - [`layer_helpers`] - Layer visibility/locking helpers
//! - [`draw_tool`] - Freehand drawing system
//! - [`line_tool`] - Straight line drawing system
//! - [`rendering`] - Gizmo rendering systems
//! - [`text_tool`] - Text annotation system (disabled)
//!
//! ## Annotation Types
//!
//! - [`DrawnPath`]: Freehand drawing paths (a series of connected points)
//! - [`DrawnLine`]: Straight lines between two points
//! - [`TextAnnotation`]: Text labels at specific positions
//!
//! ## Hit Testing
//!
//! Helper functions for detecting clicks on annotations:
//! - [`point_near_path`]: Check if a point is near a drawn path
//! - [`point_near_line`]: Check if a point is near a line
//! - [`point_in_text`]: Check if a point is inside text bounds

mod components;
mod draw_tool;
mod gizmo;
mod hit_testing;
mod layer_helpers;
mod line_tool;
mod rendering;
mod state;
mod text_tool;

// Re-exports - Components
pub use components::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};

// Re-exports - State
pub use state::{AnnotationSettings, DrawState, LineDrawState, TextEditState};

// Re-exports - Gizmo
pub use gizmo::{configure_annotation_gizmos, AnnotationGizmoGroup};

// Re-exports - Hit Testing
pub use hit_testing::{
    line_bounds, line_overlaps_rect, path_bounds, path_overlaps_rect, point_in_text,
    point_near_line, point_near_path, text_bounds, text_overlaps_rect,
};

// Re-exports - Layer Helpers
pub use layer_helpers::{is_annotation_layer_locked, is_annotation_layer_visible};

// Re-exports - Systems
pub use draw_tool::handle_draw;
pub use line_tool::handle_line;
pub use rendering::{
    render_draw_preview, render_drawn_lines, render_drawn_paths, render_line_preview,
};

// Text tool disabled - see TODO in tools.rs
#[allow(unused_imports)]
pub use rendering::render_text_annotations;
#[allow(unused_imports)]
pub use text_tool::{handle_text, text_annotation_input_ui};
