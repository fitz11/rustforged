//! Centralized constants used across the application.
//!
//! This module contains magic numbers and configuration values that are used
//! in multiple places or would benefit from being named constants.

/// Default window width in pixels (also used for grid viewport calculations)
pub const DEFAULT_WINDOW_WIDTH: f32 = 1600.0;

/// Default window height in pixels (also used for grid viewport calculations)
pub const DEFAULT_WINDOW_HEIGHT: f32 = 900.0;

/// Most thumbnails read or rendered in the background at once.
pub const MAX_THUMBNAILS_IN_FLIGHT: usize = 4;

/// Maximum number of recent libraries to remember in config
pub const MAX_RECENT_LIBRARIES: usize = 5;
