//! Common types shared across multiple modules.
//!
//! This module contains types that are used by both the editor and session modules
//! to avoid code duplication.

use bevy::window::{CursorIcon, SystemCursorIcon};

/// Drag mode for selection and viewport interaction.
///
/// Used by both the selection system (editor) and viewport indicator (session).
/// Not all modes may be applicable in all contexts (e.g., Rotate is only used
/// for selection, not viewport).
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DragMode {
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

impl DragMode {
    /// Get the appropriate cursor icon for this drag mode.
    pub fn cursor_icon(&self) -> Option<CursorIcon> {
        match self {
            DragMode::None => None,
            DragMode::Move => Some(CursorIcon::System(SystemCursorIcon::Move)),
            DragMode::Rotate => Some(CursorIcon::System(SystemCursorIcon::Grab)),
            DragMode::ResizeN | DragMode::ResizeS => {
                Some(CursorIcon::System(SystemCursorIcon::NsResize))
            }
            DragMode::ResizeE | DragMode::ResizeW => {
                Some(CursorIcon::System(SystemCursorIcon::EwResize))
            }
            DragMode::ResizeNE | DragMode::ResizeSW => {
                Some(CursorIcon::System(SystemCursorIcon::NeswResize))
            }
            DragMode::ResizeNW | DragMode::ResizeSE => {
                Some(CursorIcon::System(SystemCursorIcon::NwseResize))
            }
        }
    }

    /// Check if this is a resize mode.
    #[allow(dead_code)]
    pub fn is_resize(&self) -> bool {
        matches!(
            self,
            DragMode::ResizeN
                | DragMode::ResizeS
                | DragMode::ResizeE
                | DragMode::ResizeW
                | DragMode::ResizeNE
                | DragMode::ResizeNW
                | DragMode::ResizeSE
                | DragMode::ResizeSW
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drag_mode_default() {
        assert_eq!(DragMode::default(), DragMode::None);
    }

    #[test]
    fn test_cursor_icon_none() {
        assert!(DragMode::None.cursor_icon().is_none());
    }

    #[test]
    fn test_cursor_icon_move() {
        assert!(DragMode::Move.cursor_icon().is_some());
    }

    #[test]
    fn test_is_resize() {
        assert!(!DragMode::None.is_resize());
        assert!(!DragMode::Move.is_resize());
        assert!(!DragMode::Rotate.is_resize());
        assert!(DragMode::ResizeN.is_resize());
        assert!(DragMode::ResizeSW.is_resize());
    }
}
