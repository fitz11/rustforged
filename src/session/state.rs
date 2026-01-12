use bevy::prelude::*;

/// Information about a connected monitor
#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub name: String,
    pub physical_size: UVec2,
    pub index: usize,
}

impl MonitorInfo {
    pub fn aspect_ratio(&self) -> f32 {
        self.physical_size.x as f32 / self.physical_size.y as f32
    }
}

/// Main resource tracking the live session state
#[derive(Resource, Default)]
pub struct LiveSessionState {
    pub is_active: bool,
    pub selected_monitor: Option<MonitorInfo>,
    pub viewport_center: Vec2,
    pub viewport_size: Vec2,
    /// Rotation in 90-degree increments (0, 90, 180, 270)
    pub rotation_degrees: i32,
}

impl LiveSessionState {
    /// Get the aspect ratio of the selected monitor (defaults to 16:9)
    pub fn monitor_aspect_ratio(&self) -> f32 {
        self.selected_monitor
            .as_ref()
            .map(|m| m.aspect_ratio())
            .unwrap_or(16.0 / 9.0)
    }

    /// Get rotation in radians
    pub fn rotation_radians(&self) -> f32 {
        (self.rotation_degrees as f32).to_radians()
    }

    /// Rotate viewport by 90 degrees clockwise
    pub fn rotate_cw(&mut self) {
        self.rotation_degrees = (self.rotation_degrees + 90) % 360;
    }

    /// Rotate viewport by 90 degrees counter-clockwise
    pub fn rotate_ccw(&mut self) {
        self.rotation_degrees = (self.rotation_degrees - 90 + 360) % 360;
    }

    /// Check if viewport is in portrait orientation (90 or 270 degrees)
    pub fn is_portrait(&self) -> bool {
        self.rotation_degrees == 90 || self.rotation_degrees == 270
    }

    /// Get the effective viewport size accounting for rotation
    /// (swaps width/height when rotated 90 or 270 degrees)
    pub fn effective_viewport_size(&self) -> Vec2 {
        if self.is_portrait() {
            Vec2::new(self.viewport_size.y, self.viewport_size.x)
        } else {
            self.viewport_size
        }
    }
}

/// Drag mode for viewport interaction
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewportDragMode {
    #[default]
    None,
    Move,
    ResizeN,
    ResizeS,
    ResizeE,
    ResizeW,
    ResizeNE,
    ResizeNW,
    ResizeSE,
    ResizeSW,
}

/// Resource tracking viewport drag state
#[derive(Resource, Default)]
pub struct ViewportDragState {
    pub mode: ViewportDragMode,
    pub drag_start_world: Vec2,
    pub original_center: Vec2,
    pub original_size: Vec2,
}

/// Resource for the monitor selection dialog
#[derive(Resource, Default)]
pub struct MonitorSelectionDialog {
    pub is_open: bool,
    pub available_monitors: Vec<MonitorInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn create_test_monitor(width: u32, height: u32) -> MonitorInfo {
        MonitorInfo {
            name: "Test Monitor".to_string(),
            physical_size: UVec2::new(width, height),
            index: 0,
        }
    }

    // MonitorInfo tests
    #[test]
    fn test_monitor_aspect_ratio_16_9() {
        let monitor = create_test_monitor(1920, 1080);
        let aspect = monitor.aspect_ratio();
        assert!((aspect - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_monitor_aspect_ratio_4_3() {
        let monitor = create_test_monitor(1024, 768);
        let aspect = monitor.aspect_ratio();
        assert!((aspect - 4.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_monitor_aspect_ratio_ultrawide() {
        let monitor = create_test_monitor(3440, 1440);
        let aspect = monitor.aspect_ratio();
        assert!((aspect - 3440.0 / 1440.0).abs() < 0.01);
    }

    // LiveSessionState tests
    #[test]
    fn test_default_state() {
        let state = LiveSessionState::default();
        assert!(!state.is_active);
        assert!(state.selected_monitor.is_none());
        assert_eq!(state.viewport_center, Vec2::ZERO);
        assert_eq!(state.viewport_size, Vec2::ZERO);
        assert_eq!(state.rotation_degrees, 0);
    }

    #[test]
    fn test_monitor_aspect_ratio_defaults_to_16_9() {
        let state = LiveSessionState::default();
        let aspect = state.monitor_aspect_ratio();
        assert!((aspect - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn test_monitor_aspect_ratio_uses_selected_monitor() {
        let state = LiveSessionState {
            selected_monitor: Some(create_test_monitor(1024, 768)),
            ..Default::default()
        };
        let aspect = state.monitor_aspect_ratio();
        assert!((aspect - 4.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_rotation_radians_0() {
        let state = LiveSessionState::default();
        assert_eq!(state.rotation_radians(), 0.0);
    }

    #[test]
    fn test_rotation_radians_90() {
        let state = LiveSessionState {
            rotation_degrees: 90,
            ..Default::default()
        };
        assert!((state.rotation_radians() - PI / 2.0).abs() < 0.001);
    }

    #[test]
    fn test_rotation_radians_180() {
        let state = LiveSessionState {
            rotation_degrees: 180,
            ..Default::default()
        };
        assert!((state.rotation_radians() - PI).abs() < 0.001);
    }

    #[test]
    fn test_rotation_radians_270() {
        let state = LiveSessionState {
            rotation_degrees: 270,
            ..Default::default()
        };
        assert!((state.rotation_radians() - 3.0 * PI / 2.0).abs() < 0.001);
    }

    #[test]
    fn test_rotate_cw_from_0() {
        let mut state = LiveSessionState::default();
        state.rotate_cw();
        assert_eq!(state.rotation_degrees, 90);
    }

    #[test]
    fn test_rotate_cw_wraps_around() {
        let mut state = LiveSessionState {
            rotation_degrees: 270,
            ..Default::default()
        };
        state.rotate_cw();
        assert_eq!(state.rotation_degrees, 0);
    }

    #[test]
    fn test_rotate_cw_full_cycle() {
        let mut state = LiveSessionState::default();
        state.rotate_cw();
        assert_eq!(state.rotation_degrees, 90);
        state.rotate_cw();
        assert_eq!(state.rotation_degrees, 180);
        state.rotate_cw();
        assert_eq!(state.rotation_degrees, 270);
        state.rotate_cw();
        assert_eq!(state.rotation_degrees, 0);
    }

    #[test]
    fn test_rotate_ccw_from_0() {
        let mut state = LiveSessionState::default();
        state.rotate_ccw();
        assert_eq!(state.rotation_degrees, 270);
    }

    #[test]
    fn test_rotate_ccw_from_90() {
        let mut state = LiveSessionState {
            rotation_degrees: 90,
            ..Default::default()
        };
        state.rotate_ccw();
        assert_eq!(state.rotation_degrees, 0);
    }

    #[test]
    fn test_rotate_ccw_full_cycle() {
        let mut state = LiveSessionState::default();
        state.rotate_ccw();
        assert_eq!(state.rotation_degrees, 270);
        state.rotate_ccw();
        assert_eq!(state.rotation_degrees, 180);
        state.rotate_ccw();
        assert_eq!(state.rotation_degrees, 90);
        state.rotate_ccw();
        assert_eq!(state.rotation_degrees, 0);
    }

    #[test]
    fn test_is_portrait() {
        let mut state = LiveSessionState {
            rotation_degrees: 0,
            ..Default::default()
        };
        assert!(!state.is_portrait());

        state.rotation_degrees = 90;
        assert!(state.is_portrait());

        state.rotation_degrees = 180;
        assert!(!state.is_portrait());

        state.rotation_degrees = 270;
        assert!(state.is_portrait());
    }

    #[test]
    fn test_effective_viewport_size_landscape() {
        let state = LiveSessionState {
            viewport_size: Vec2::new(1920.0, 1080.0),
            rotation_degrees: 0,
            ..Default::default()
        };

        let effective = state.effective_viewport_size();
        assert_eq!(effective, Vec2::new(1920.0, 1080.0));
    }

    #[test]
    fn test_effective_viewport_size_portrait_90() {
        let state = LiveSessionState {
            viewport_size: Vec2::new(1920.0, 1080.0),
            rotation_degrees: 90,
            ..Default::default()
        };

        let effective = state.effective_viewport_size();
        assert_eq!(effective, Vec2::new(1080.0, 1920.0));
    }

    #[test]
    fn test_effective_viewport_size_180() {
        let state = LiveSessionState {
            viewport_size: Vec2::new(1920.0, 1080.0),
            rotation_degrees: 180,
            ..Default::default()
        };

        let effective = state.effective_viewport_size();
        assert_eq!(effective, Vec2::new(1920.0, 1080.0));
    }

    #[test]
    fn test_effective_viewport_size_portrait_270() {
        let state = LiveSessionState {
            viewport_size: Vec2::new(1920.0, 1080.0),
            rotation_degrees: 270,
            ..Default::default()
        };

        let effective = state.effective_viewport_size();
        assert_eq!(effective, Vec2::new(1080.0, 1920.0));
    }

    // ViewportDragMode tests
    #[test]
    fn test_viewport_drag_mode_default() {
        assert_eq!(ViewportDragMode::default(), ViewportDragMode::None);
    }

    // ViewportDragState tests
    #[test]
    fn test_viewport_drag_state_default() {
        let state = ViewportDragState::default();
        assert_eq!(state.mode, ViewportDragMode::None);
        assert_eq!(state.drag_start_world, Vec2::ZERO);
        assert_eq!(state.original_center, Vec2::ZERO);
        assert_eq!(state.original_size, Vec2::ZERO);
    }

    // MonitorSelectionDialog tests
    #[test]
    fn test_monitor_selection_dialog_default() {
        let dialog = MonitorSelectionDialog::default();
        assert!(!dialog.is_open);
        assert!(dialog.available_monitors.is_empty());
    }
}
