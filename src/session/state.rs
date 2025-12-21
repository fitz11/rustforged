use bevy::prelude::*;

/// Information about a connected monitor
#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub name: String,
    pub physical_size: UVec2,
    pub scale_factor: f64,
    pub position: IVec2,
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

    /// Get the viewport bounds as (min, max) corners (unrotated)
    pub fn viewport_bounds(&self) -> (Vec2, Vec2) {
        let half_size = self.viewport_size / 2.0;
        (
            self.viewport_center - half_size,
            self.viewport_center + half_size,
        )
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
