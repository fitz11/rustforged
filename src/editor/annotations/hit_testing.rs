//! Hit testing functions for detecting clicks on annotations.

use bevy::prelude::*;

use super::components::{DrawnLine, DrawnPath, TextAnnotation};

/// Check if a point is within a given distance of a line segment
fn point_near_segment(point: Vec2, seg_start: Vec2, seg_end: Vec2, threshold: f32) -> bool {
    let line_vec = seg_end - seg_start;
    let line_len_sq = line_vec.length_squared();

    if line_len_sq < 0.0001 {
        // Segment is essentially a point
        return point.distance(seg_start) <= threshold;
    }

    // Project point onto line, clamped to segment
    let t = ((point - seg_start).dot(line_vec) / line_len_sq).clamp(0.0, 1.0);
    let projection = seg_start + line_vec * t;

    point.distance(projection) <= threshold
}

/// Check if a point is near a drawn path
pub fn point_near_path(point: Vec2, path: &DrawnPath) -> bool {
    let threshold = (path.stroke_width * 2.0).max(8.0); // Hit area is at least 8px

    for window in path.points.windows(2) {
        if point_near_segment(point, window[0], window[1], threshold) {
            return true;
        }
    }
    false
}

/// Check if a point is near a drawn line
pub fn point_near_line(point: Vec2, line: &DrawnLine) -> bool {
    let threshold = (line.stroke_width * 2.0).max(8.0);
    point_near_segment(point, line.start, line.end, threshold)
}

/// Check if a point is inside a text annotation's bounding box
pub fn point_in_text(point: Vec2, transform: &Transform, text: &TextAnnotation) -> bool {
    let pos = transform.translation.truncate();
    let width = (text.content.len() as f32 * text.font_size * 0.5).max(40.0);
    let height = text.font_size.max(20.0);
    let half_size = Vec2::new(width / 2.0, height / 2.0);

    (point.x - pos.x).abs() < half_size.x && (point.y - pos.y).abs() < half_size.y
}

/// Get the bounding box of a path (min, max corners)
pub fn path_bounds(path: &DrawnPath) -> (Vec2, Vec2) {
    if path.points.is_empty() {
        return (Vec2::ZERO, Vec2::ZERO);
    }

    let mut min = path.points[0];
    let mut max = path.points[0];

    for &p in &path.points {
        min = min.min(p);
        max = max.max(p);
    }

    // Expand by stroke width
    let padding = path.stroke_width;
    (min - Vec2::splat(padding), max + Vec2::splat(padding))
}

/// Get the bounding box of a line (min, max corners)
pub fn line_bounds(line: &DrawnLine) -> (Vec2, Vec2) {
    let min = line.start.min(line.end);
    let max = line.start.max(line.end);
    let padding = line.stroke_width;
    (min - Vec2::splat(padding), max + Vec2::splat(padding))
}

/// Get the bounding box of a text annotation (min, max corners)
pub fn text_bounds(transform: &Transform, text: &TextAnnotation) -> (Vec2, Vec2) {
    let pos = transform.translation.truncate();
    let width = (text.content.len() as f32 * text.font_size * 0.5).max(40.0);
    let height = text.font_size.max(20.0);
    let half_size = Vec2::new(width / 2.0, height / 2.0);

    (pos - half_size, pos + half_size)
}

/// Check if a path overlaps with a selection rectangle
pub fn path_overlaps_rect(rect_min: Vec2, rect_max: Vec2, path: &DrawnPath) -> bool {
    let (path_min, path_max) = path_bounds(path);
    rect_min.x < path_max.x
        && rect_max.x > path_min.x
        && rect_min.y < path_max.y
        && rect_max.y > path_min.y
}

/// Check if a line overlaps with a selection rectangle
pub fn line_overlaps_rect(rect_min: Vec2, rect_max: Vec2, line: &DrawnLine) -> bool {
    let (line_min, line_max) = line_bounds(line);
    rect_min.x < line_max.x
        && rect_max.x > line_min.x
        && rect_min.y < line_max.y
        && rect_max.y > line_min.y
}

/// Check if a text annotation overlaps with a selection rectangle
pub fn text_overlaps_rect(
    rect_min: Vec2,
    rect_max: Vec2,
    transform: &Transform,
    text: &TextAnnotation,
) -> bool {
    let (text_min, text_max) = text_bounds(transform, text);
    rect_min.x < text_max.x
        && rect_max.x > text_min.x
        && rect_min.y < text_max.y
        && rect_max.y > text_min.y
}
