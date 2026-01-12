//! Hit detection and geometry helpers for selection.

use bevy::prelude::*;

use crate::editor::annotations::{
    point_in_text, point_near_line, point_near_path, AnnotationMarker, DrawnLine, DrawnPath,
    TextAnnotation,
};
use crate::map::Selected;

use super::{SelectionDragMode, HANDLE_SIZE, ROTATION_HANDLE_OFFSET, ROTATION_HANDLE_RADIUS};

/// Rotate a point around a center by the given angle (in radians)
pub(crate) fn rotate_point(point: Vec2, center: Vec2, angle: f32) -> Vec2 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let translated = point - center;
    Vec2::new(
        translated.x * cos_a - translated.y * sin_a,
        translated.x * sin_a + translated.y * cos_a,
    ) + center
}

/// Get the half-size of a sprite, accounting for custom_size or image dimensions
pub(crate) fn get_sprite_half_size(sprite: &Sprite, images: &Assets<Image>) -> Vec2 {
    // Check for custom_size first
    if let Some(custom_size) = sprite.custom_size {
        return custom_size / 2.0;
    }

    // Try to get size from the image
    if let Some(image) = images.get(&sprite.image) {
        let size = image.size().as_vec2();
        return size / 2.0;
    }

    // Fallback to a default size
    Vec2::splat(32.0)
}

/// Check if a point is inside an item's bounds, accounting for rotation
pub(crate) fn point_in_item(
    world_pos: Vec2,
    transform: &Transform,
    sprite: &Sprite,
    images: &Assets<Image>,
) -> bool {
    let item_pos = transform.translation.truncate();
    let half_size = get_sprite_half_size(sprite, images) * transform.scale.truncate();

    // Transform the world position into the item's local coordinate space
    // by applying the inverse rotation
    let diff = world_pos - item_pos;
    // EulerRot::ZYX returns (z, y, x) - we want the Z rotation (first component)
    let (angle, _, _) = transform.rotation.to_euler(EulerRot::ZYX);
    let cos_a = (-angle).cos();
    let sin_a = (-angle).sin();
    let local_diff = Vec2::new(
        diff.x * cos_a - diff.y * sin_a,
        diff.x * sin_a + diff.y * cos_a,
    );

    local_diff.x.abs() < half_size.x && local_diff.y.abs() < half_size.y
}

/// Check if an item overlaps with a rectangle (defined by two corners)
pub(crate) fn item_overlaps_rect(
    rect_min: Vec2,
    rect_max: Vec2,
    transform: &Transform,
    sprite: &Sprite,
    images: &Assets<Image>,
) -> bool {
    let item_pos = transform.translation.truncate();
    let half_size = get_sprite_half_size(sprite, images) * transform.scale.truncate();

    let item_min = item_pos - half_size;
    let item_max = item_pos + half_size;

    // Check for overlap (AABB intersection)
    rect_min.x < item_max.x
        && rect_max.x > item_min.x
        && rect_min.y < item_max.y
        && rect_max.y > item_min.y
}

/// Compute the combined bounding box for all selected placed items
pub fn compute_selection_bounds(
    selected_query: &Query<(&Transform, &Sprite), With<Selected>>,
    images: &Assets<Image>,
) -> Option<(Vec2, Vec2)> {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    let mut found_any = false;

    for (transform, sprite) in selected_query.iter() {
        let pos = transform.translation.truncate();
        let half_size = get_sprite_half_size(sprite, images) * transform.scale.truncate();
        let item_min = pos - half_size;
        let item_max = pos + half_size;

        min = min.min(item_min);
        max = max.max(item_max);
        found_any = true;
    }

    if found_any {
        Some((min, max))
    } else {
        None
    }
}

/// Check if the cursor is over any selected item's rotation handle (accounting for rotation)
pub fn check_rotation_handle_hit(
    world_pos: Vec2,
    camera_scale: f32,
    selected_query: &Query<(&Transform, &Sprite), With<Selected>>,
    images: &Assets<Image>,
) -> bool {
    let rotation_hit_size = ROTATION_HANDLE_RADIUS * camera_scale * 1.5;

    for (transform, sprite) in selected_query.iter() {
        let pos = transform.translation.truncate();
        let half_size = get_sprite_half_size(sprite, images) * transform.scale.truncate();
        let (angle, _, _) = transform.rotation.to_euler(EulerRot::ZYX);

        // Calculate the rotated rotation handle position
        let local_handle = Vec2::new(0.0, half_size.y + ROTATION_HANDLE_OFFSET);
        let world_handle = rotate_point(pos + local_handle, pos, angle);

        if (world_pos - world_handle).length() < rotation_hit_size {
            return true;
        }
    }

    false
}

/// Determine which handle (if any) is under the cursor for selected items
/// This version accounts for item rotation by checking rotated handle positions
pub fn get_selection_handle_at_position(
    world_pos: Vec2,
    selected_query: &Query<(&Transform, &Sprite), With<Selected>>,
    images: &Assets<Image>,
    camera_scale: f32,
) -> SelectionDragMode {
    // Adjust handle hit area based on camera zoom
    let hit_size = HANDLE_SIZE * camera_scale * 1.5;

    // Check each selected item's rotated handles
    for (transform, sprite) in selected_query.iter() {
        let pos = transform.translation.truncate();
        let half_size = get_sprite_half_size(sprite, images) * transform.scale.truncate();
        let (angle, _, _) = transform.rotation.to_euler(EulerRot::ZYX);

        // Local corner positions (before rotation)
        let local_corners = [
            (Vec2::new(-half_size.x, -half_size.y), SelectionDragMode::ResizeSW),
            (Vec2::new(half_size.x, -half_size.y), SelectionDragMode::ResizeSE),
            (Vec2::new(half_size.x, half_size.y), SelectionDragMode::ResizeNE),
            (Vec2::new(-half_size.x, half_size.y), SelectionDragMode::ResizeNW),
        ];

        // Check corners (high priority)
        for (local_corner, mode) in local_corners {
            let world_corner = rotate_point(pos + local_corner, pos, angle);
            if (world_pos - world_corner).length() < hit_size {
                return mode;
            }
        }

        // Local edge positions (before rotation)
        let local_edges = [
            (Vec2::new(0.0, -half_size.y), SelectionDragMode::ResizeS),
            (Vec2::new(0.0, half_size.y), SelectionDragMode::ResizeN),
            (Vec2::new(-half_size.x, 0.0), SelectionDragMode::ResizeW),
            (Vec2::new(half_size.x, 0.0), SelectionDragMode::ResizeE),
        ];

        // Check edge handles
        for (local_edge, mode) in local_edges {
            let world_edge = rotate_point(pos + local_edge, pos, angle);
            if (world_pos - world_edge).length() < hit_size {
                return mode;
            }
        }

        // Check if inside the rotated selection rectangle (for move/grab)
        // Transform cursor position to item's local space
        if point_in_item(world_pos, transform, sprite, images) {
            return SelectionDragMode::Move;
        }
    }

    SelectionDragMode::None
}

/// Find which annotation (if any) was clicked
pub(crate) fn find_clicked_annotation(
    world_pos: Vec2,
    paths_query: &Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    lines_query: &Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    texts_query: &Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) -> Option<Entity> {
    // Check text annotations first (they have clear bounds)
    for (entity, transform, text) in texts_query.iter() {
        if point_in_text(world_pos, transform, text) {
            return Some(entity);
        }
    }

    // Check lines
    for (entity, line) in lines_query.iter() {
        if point_near_line(world_pos, line) {
            return Some(entity);
        }
    }

    // Check paths
    for (entity, path) in paths_query.iter() {
        if point_near_path(world_pos, path) {
            return Some(entity);
        }
    }

    None
}
