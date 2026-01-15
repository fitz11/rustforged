//! Helper functions for clipboard operations.

use bevy::prelude::*;

use crate::map::{SavedPath, Selected};

use super::super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use crate::map::PlacedItem;

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

/// Calculate the centroid of all selected items
#[allow(clippy::type_complexity)]
pub fn calculate_selection_centroid(
    placed_items: &Query<(&PlacedItem, &Transform), With<Selected>>,
    paths: &Query<&DrawnPath, (With<Selected>, With<AnnotationMarker>)>,
    lines: &Query<&DrawnLine, (With<Selected>, With<AnnotationMarker>)>,
    texts: &Query<(&Transform, &TextAnnotation), (With<Selected>, With<AnnotationMarker>)>,
) -> Vec2 {
    let mut positions: Vec<Vec2> = Vec::new();

    // Collect placed item positions
    for (_, transform) in placed_items.iter() {
        positions.push(transform.translation.truncate());
    }

    // Collect path centers
    for path in paths.iter() {
        positions.push(path_center(path));
    }

    // Collect line centers
    for line in lines.iter() {
        positions.push((line.start + line.end) / 2.0);
    }

    // Collect text positions
    for (transform, _) in texts.iter() {
        positions.push(transform.translation.truncate());
    }

    if positions.is_empty() {
        return Vec2::ZERO;
    }

    let sum: Vec2 = positions.iter().copied().sum();
    sum / positions.len() as f32
}
