use bevy::prelude::*;

use crate::common::DragMode;

/// Type alias for viewport drag mode (uses common DragMode)
pub type ViewportDragMode = DragMode;

/// Information about a connected monitor
#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub name: String,
    pub physical_size: UVec2,
    /// Used on macOS to manually position the player window (avoids Space creation)
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub physical_position: IVec2,
    /// Used on macOS to set the correct scale factor for the player window
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub scale_factor: f64,
    /// The Bevy entity for this monitor (used for BorderlessFullscreen on non-macOS)
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub entity: Option<Entity>,
    /// Scale factor of the primary monitor (needed for macOS position correction)
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub primary_scale_factor: f64,
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
    /// Players keep seeing the map as it was when this was turned on
    pub frozen: bool,
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
}

/// The part of the map the player window shows. Follows the session viewport,
/// except while the player view is frozen in place.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct PlayerViewport {
    pub center: Vec2,
    pub size: Vec2,
    /// Rotation in 90-degree increments (0, 90, 180, 270)
    pub rotation_degrees: i32,
}

impl PlayerViewport {
    /// The session's current viewport
    pub fn of(session_state: &LiveSessionState) -> Self {
        Self {
            center: session_state.viewport_center,
            size: session_state.viewport_size,
            rotation_degrees: session_state.rotation_degrees,
        }
    }

    pub fn rotation_radians(&self) -> f32 {
        (self.rotation_degrees as f32).to_radians()
    }

    /// Axis-aligned world bounds (min, max) of the viewport rectangle after
    /// rotation about its center
    pub fn world_bounds(&self) -> (Vec2, Vec2) {
        let half = self.size / 2.0;
        let rotation = Rot2::radians(self.rotation_radians());
        let corners = [
            Vec2::new(-half.x, -half.y),
            Vec2::new(half.x, -half.y),
            Vec2::new(half.x, half.y),
            Vec2::new(-half.x, half.y),
        ]
        .map(|corner| self.center + rotation * corner);

        let min = corners.into_iter().reduce(Vec2::min).unwrap_or(self.center);
        let max = corners.into_iter().reduce(Vec2::max).unwrap_or(self.center);
        (min, max)
    }
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
            physical_position: IVec2::ZERO,
            scale_factor: 1.0,
            entity: None,
            primary_scale_factor: 1.0,
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
        assert!(!state.frozen);
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

    // PlayerViewport tests
    #[test]
    fn test_player_viewport_of_session() {
        let state = LiveSessionState {
            viewport_center: Vec2::new(10.0, 20.0),
            viewport_size: Vec2::new(400.0, 300.0),
            rotation_degrees: 90,
            ..Default::default()
        };
        let viewport = PlayerViewport::of(&state);
        assert_eq!(viewport.center, Vec2::new(10.0, 20.0));
        assert_eq!(viewport.size, Vec2::new(400.0, 300.0));
        assert_eq!(viewport.rotation_degrees, 90);
    }

    fn assert_near(actual: Vec2, expected: Vec2) {
        assert!(
            (actual - expected).length() < 0.001,
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn test_player_viewport_world_bounds_unrotated() {
        let viewport = PlayerViewport {
            center: Vec2::new(100.0, 50.0),
            size: Vec2::new(400.0, 300.0),
            rotation_degrees: 0,
        };
        let (min, max) = viewport.world_bounds();
        assert_near(min, Vec2::new(-100.0, -100.0));
        assert_near(max, Vec2::new(300.0, 200.0));
    }

    #[test]
    fn test_player_viewport_world_bounds_quarter_turn_swaps_extents() {
        let viewport = PlayerViewport {
            center: Vec2::ZERO,
            size: Vec2::new(400.0, 300.0),
            rotation_degrees: 90,
        };
        let (min, max) = viewport.world_bounds();
        assert_near(min, Vec2::new(-150.0, -200.0));
        assert_near(max, Vec2::new(150.0, 200.0));
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
