//! Centralized color theme for the application.
//!
//! This module provides all colors used throughout the editor UI and rendering.
//! Modify values here to change the application's color scheme.

use bevy::prelude::Color;
use bevy_egui::egui;

// ============================================================================
// Grid Colors
// ============================================================================

/// Semi-transparent grey grid lines
pub const GRID_COLOR: Color = Color::srgba(0.5, 0.5, 0.5, 0.3);

// ============================================================================
// Selection Colors
// ============================================================================

/// Light blue for selection rectangles and indicators
pub const SELECTION_COLOR: Color = Color::srgb(0.2, 0.6, 1.0);

/// Blue outline for box selection
pub const BOX_SELECT_OUTLINE: Color = Color::srgba(0.2, 0.6, 1.0, 0.8);

/// Very light blue fill for box selection
pub const BOX_SELECT_FILL: Color = Color::srgba(0.2, 0.6, 1.0, 0.1);

// ============================================================================
// Viewport Indicator Colors
// ============================================================================

/// Orange outline for the player viewport indicator
pub const VIEWPORT_OUTLINE: Color = Color::srgba(1.0, 0.7, 0.2, 0.9);

/// Light orange fill for viewport indicator
pub const VIEWPORT_FILL: Color = Color::srgba(1.0, 0.7, 0.2, 0.1);

/// Opaque orange for the viewport move handle
pub const VIEWPORT_HANDLE: Color = Color::srgba(1.0, 0.7, 0.2, 1.0);

// ============================================================================
// Annotation Colors
// ============================================================================

/// Default annotation stroke color (red)
pub const ANNOTATION_DEFAULT: Color = Color::srgb(1.0, 0.0, 0.0);

/// Annotation color palette for the toolbar picker
pub fn annotation_colors() -> [(Color, &'static str, egui::Color32); 8] {
    [
        (Color::srgb(1.0, 0.0, 0.0), "Red", egui::Color32::RED),
        (Color::srgb(0.0, 0.0, 1.0), "Blue", egui::Color32::BLUE),
        (
            Color::srgb(0.0, 0.8, 0.0),
            "Green",
            egui::Color32::from_rgb(0, 200, 0),
        ),
        (Color::srgb(1.0, 1.0, 0.0), "Yellow", egui::Color32::YELLOW),
        (Color::srgb(0.0, 0.0, 0.0), "Black", egui::Color32::BLACK),
        (Color::srgb(1.0, 1.0, 1.0), "White", egui::Color32::WHITE),
        (
            Color::srgb(1.0, 0.5, 0.0),
            "Orange",
            egui::Color32::from_rgb(255, 128, 0),
        ),
        (
            Color::srgb(0.5, 0.0, 0.5),
            "Purple",
            egui::Color32::from_rgb(128, 0, 128),
        ),
    ]
}

// ============================================================================
// Fog of War Colors
// ============================================================================

/// Base fog color (dark blue-black) for editor view
/// Alpha is controlled by FogState.editor_opacity
pub const FOG_EDITOR_BASE: Color = Color::srgb(0.05, 0.05, 0.12);

/// Yellow highlight for fog brush preview (single cell mode)
pub const FOG_BRUSH_CELL_HIGHLIGHT: Color = Color::srgba(1.0, 1.0, 0.0, 0.5);

/// Yellow circle for fog brush preview (circular mode)
pub const FOG_BRUSH_CIRCLE: Color = Color::srgba(1.0, 1.0, 0.0, 0.8);

/// Opaque black fog for player view
pub const FOG_PLAYER: Color = Color::BLACK;

// ============================================================================
// Asset Validation Colors
// ============================================================================

/// Red border indicator for missing/broken assets
pub const MISSING_ASSET_BORDER: Color = Color::srgba(1.0, 0.3, 0.3, 0.9);

// ============================================================================
// Player Window
// ============================================================================

/// Background color for player window
pub const PLAYER_BACKGROUND: Color = Color::BLACK;

// ============================================================================
// UI Colors (egui)
// ============================================================================

pub mod ui {
    use bevy_egui::egui;

    /// Green "LIVE" session indicator
    pub const SESSION_ACTIVE: egui::Color32 = egui::Color32::from_rgb(100, 200, 100);

    /// Dark grey panel background (tool settings bar)
    pub const PANEL_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(45, 45, 48);

    /// Light grey for label text
    pub const LABEL_TEXT: egui::Color32 = egui::Color32::LIGHT_GRAY;

    /// Grey for help/hint text
    pub const HINT_TEXT: egui::Color32 = egui::Color32::GRAY;

    /// White for selected button borders
    pub const SELECTED_BORDER: egui::Color32 = egui::Color32::WHITE;

    /// Dark grey for unselected button borders
    pub const UNSELECTED_BORDER: egui::Color32 = egui::Color32::DARK_GRAY;

    /// Red for error messages
    pub const ERROR_TEXT: egui::Color32 = egui::Color32::RED;

    /// Pinkish-red for missing asset text
    pub const MISSING_ASSET_TEXT: egui::Color32 = egui::Color32::from_rgb(200, 100, 100);

    /// Semi-transparent black overlay for modal dialogs
    pub const MODAL_OVERLAY: egui::Color32 = egui::Color32::from_black_alpha(100);

    /// Dark background for asset browser panels
    pub const ASSET_BROWSER_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(60, 60, 60);

    /// File extension badge colors
    pub mod file_ext {
        use bevy_egui::egui;

        pub const PNG: egui::Color32 = egui::Color32::from_rgb(80, 140, 200);
        pub const JPG: egui::Color32 = egui::Color32::from_rgb(200, 140, 80);
        pub const WEBP: egui::Color32 = egui::Color32::from_rgb(140, 200, 80);
        pub const GIF: egui::Color32 = egui::Color32::from_rgb(200, 80, 140);
        pub const BMP: egui::Color32 = egui::Color32::from_rgb(140, 80, 200);
        pub const TIFF: egui::Color32 = egui::Color32::from_rgb(80, 200, 140);
        pub const DEFAULT: egui::Color32 = egui::Color32::from_rgb(128, 128, 128);

        /// Get color for file extension
        pub fn color_for(ext: &str) -> egui::Color32 {
            match ext.to_lowercase().as_str() {
                "png" => PNG,
                "jpg" | "jpeg" => JPG,
                "webp" => WEBP,
                "gif" => GIF,
                "bmp" => BMP,
                "tiff" | "tif" => TIFF,
                _ => DEFAULT,
            }
        }
    }
}

// ============================================================================
// Color Conversion Utilities
// ============================================================================

/// Convert a Bevy Color to egui Color32 (fully opaque)
pub fn bevy_to_egui_opaque(color: Color) -> egui::Color32 {
    let srgba = color.to_srgba();
    egui::Color32::from_rgba_unmultiplied(
        (srgba.red * 255.0) as u8,
        (srgba.green * 255.0) as u8,
        (srgba.blue * 255.0) as u8,
        255,
    )
}

/// Convert a Bevy Color to egui Color32 (preserving alpha)
pub fn bevy_to_egui(color: Color) -> egui::Color32 {
    let srgba = color.to_srgba();
    egui::Color32::from_rgba_unmultiplied(
        (srgba.red * 255.0) as u8,
        (srgba.green * 255.0) as u8,
        (srgba.blue * 255.0) as u8,
        (srgba.alpha * 255.0) as u8,
    )
}

/// Convert an egui Color32 to Bevy Color
pub fn egui_to_bevy(color: egui::Color32) -> Color {
    Color::srgba(
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
        color.a() as f32 / 255.0,
    )
}
