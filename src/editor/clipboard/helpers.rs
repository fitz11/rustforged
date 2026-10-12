//! Helper functions for clipboard operations.

use bevy::prelude::*;

use crate::map::SavedPath;

use super::super::annotations::DrawnPath;

/// Convert Color to [f32; 4] array for saved formats
pub fn color_to_array(color: Color) -> [f32; 4] {
    let srgba = color.to_srgba();
    [srgba.red, srgba.green, srgba.blue, srgba.alpha]
}

/// Convert [f32; 4] array to Color
pub fn array_to_color(arr: [f32; 4]) -> Color {
    Color::srgba(arr[0], arr[1], arr[2], arr[3])
}

/// Calculate the center of a DrawnPath
pub fn path_center(path: &DrawnPath) -> Vec2 {
    if path.points.is_empty() {
        return Vec2::ZERO;
    }
    let sum: Vec2 = path.points.iter().copied().sum();
    sum / path.points.len() as f32
}

/// Calculate the center from saved path points
pub fn saved_path_center(saved: &SavedPath) -> Vec2 {
    if saved.points.is_empty() {
        return Vec2::ZERO;
    }
    let sum: Vec2 = saved.points.iter().copied().sum();
    sum / saved.points.len() as f32
}
