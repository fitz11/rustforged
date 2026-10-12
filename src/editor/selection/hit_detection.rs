//! Hit detection and geometry helpers for selection.

use bevy::prelude::*;

use crate::editor::annotations::{
    AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation, point_in_text, point_near_line,
    point_near_path,
};
use crate::map::Selected;

use super::resize::handle_position;
use super::{HandleMetrics, SelectionDragMode};

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

/// Half an item's size in its own frame: sprite size times scale, positive even for a
/// mirrored (negatively scaled) item
pub(crate) fn item_half_size(
    transform: &Transform,
    sprite: &Sprite,
    images: &Assets<Image>,
) -> Vec2 {
    (get_sprite_half_size(sprite, images) * transform.scale.truncate()).abs()
}

/// Half the size of the axis-aligned box around an item, accounting for its rotation
fn rotated_half_extents(transform: &Transform, half_size: Vec2) -> Vec2 {
    let (angle, _, _) = transform.rotation.to_euler(EulerRot::ZYX);
    let (sin, cos) = (angle.sin().abs(), angle.cos().abs());
    Vec2::new(
        cos * half_size.x + sin * half_size.y,
        sin * half_size.x + cos * half_size.y,
    )
}

/// The world-space box (min, max) around an item, accounting for its rotation
pub(crate) fn item_world_bounds(
    transform: &Transform,
    sprite: &Sprite,
    images: &Assets<Image>,
) -> (Vec2, Vec2) {
    let pos = transform.translation.truncate();
    let half = rotated_half_extents(transform, item_half_size(transform, sprite, images));
    (pos - half, pos + half)
}

/// Check if a point is inside an item's bounds, accounting for rotation
pub(crate) fn point_in_item(
    world_pos: Vec2,
    transform: &Transform,
    sprite: &Sprite,
    images: &Assets<Image>,
) -> bool {
    let item_pos = transform.translation.truncate();
    let half_size = item_half_size(transform, sprite, images);

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
    let half_extents = rotated_half_extents(transform, item_half_size(transform, sprite, images));

    let item_min = item_pos - half_extents;
    let item_max = item_pos + half_extents;

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
        let half_extents =
            rotated_half_extents(transform, item_half_size(transform, sprite, images));
        let item_min = pos - half_extents;
        let item_max = pos + half_extents;

        min = min.min(item_min);
        max = max.max(item_max);
        found_any = true;
    }

    if found_any { Some((min, max)) } else { None }
}

/// Where an item's rotation handle is, in world space
pub(crate) fn rotation_handle_position(
    transform: &Transform,
    half_size: Vec2,
    metrics: &HandleMetrics,
) -> Vec2 {
    let pos = transform.translation.truncate();
    let (angle, _, _) = transform.rotation.to_euler(EulerRot::ZYX);
    let local_handle = Vec2::new(0.0, half_size.y + metrics.rotation_offset);
    rotate_point(pos + local_handle, pos, angle)
}

/// The resize handles an item shows at this zoom, with where each is in world space.
/// Small items on screen show fewer (see [`HandleMetrics`]).
pub(crate) fn resize_handles(
    transform: &Transform,
    half_size: Vec2,
    metrics: &HandleMetrics,
) -> Vec<(SelectionDragMode, Vec2)> {
    const CORNERS: [SelectionDragMode; 4] = [
        SelectionDragMode::ResizeSW,
        SelectionDragMode::ResizeSE,
        SelectionDragMode::ResizeNE,
        SelectionDragMode::ResizeNW,
    ];
    const EDGES: [SelectionDragMode; 4] = [
        SelectionDragMode::ResizeS,
        SelectionDragMode::ResizeN,
        SelectionDragMode::ResizeW,
        SelectionDragMode::ResizeE,
    ];
    let pos = transform.translation.truncate();
    let (angle, _, _) = transform.rotation.to_euler(EulerRot::ZYX);
    let mut modes = Vec::new();
    if metrics.shows_corner_handles(half_size) {
        modes.extend(CORNERS);
    }
    if metrics.shows_edge_handles(half_size) {
        modes.extend(EDGES);
    }
    modes
        .into_iter()
        .map(|mode| {
            let local = handle_position(mode, half_size);
            (mode, rotate_point(pos + local, pos, angle))
        })
        .collect()
}

/// Check if the cursor is over any selected item's rotation handle (accounting for rotation)
pub fn check_rotation_handle_hit(
    world_pos: Vec2,
    camera_scale: f32,
    selected_query: &Query<(&Transform, &Sprite), With<Selected>>,
    images: &Assets<Image>,
) -> bool {
    let metrics = HandleMetrics::at_zoom(camera_scale);
    let hit_radius = metrics.rotation_radius * 1.5;
    selected_query.iter().any(|(transform, sprite)| {
        let half_size = item_half_size(transform, sprite, images);
        rotation_handle_position(transform, half_size, &metrics).distance(world_pos) < hit_radius
    })
}

/// Determine which handle (if any) is under the cursor for selected items
/// This version accounts for item rotation by checking rotated handle positions
pub fn get_selection_handle_at_position(
    world_pos: Vec2,
    selected_query: &Query<(&Transform, &Sprite), With<Selected>>,
    images: &Assets<Image>,
    camera_scale: f32,
) -> SelectionDragMode {
    let metrics = HandleMetrics::at_zoom(camera_scale);

    // Check each selected item's rotated handles, corners first
    for (transform, sprite) in selected_query.iter() {
        let half_size = item_half_size(transform, sprite, images);
        let grabbed = resize_handles(transform, half_size, &metrics)
            .into_iter()
            .find(|(_, handle)| handle.distance(world_pos) < metrics.hit_radius);
        if let Some((mode, _)) = grabbed {
            return mode;
        }

        // Inside the rotated item: move it
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite(size: f32) -> Sprite {
        Sprite {
            custom_size: Some(Vec2::splat(size)),
            ..default()
        }
    }

    #[test]
    fn test_mirrored_item_can_be_clicked() {
        let images = Assets::<Image>::default();
        let transform = Transform::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        assert!(point_in_item(
            Vec2::new(10.0, 10.0),
            &transform,
            &sprite(70.0),
            &images
        ));
    }

    #[test]
    fn test_zoomed_out_small_item_has_no_handles_over_its_middle() {
        let images = Assets::<Image>::default();
        let token = sprite(70.0);
        let transform = Transform::default();
        let half = item_half_size(&transform, &token, &images);

        // Zoomed out 3x: 70 world units are about 23 pixels on screen
        let metrics = HandleMetrics::at_zoom(3.0);
        assert!(resize_handles(&transform, half, &metrics).is_empty());

        // At normal zoom it has all eight
        assert_eq!(
            resize_handles(&transform, half, &HandleMetrics::at_zoom(1.0)).len(),
            8
        );
    }

    #[test]
    fn test_rotated_item_overlaps_by_its_rotated_box() {
        let images = Assets::<Image>::default();
        // A 100x10 bar turned 90 degrees reaches 50 up and down
        let bar = Sprite {
            custom_size: Some(Vec2::new(100.0, 10.0)),
            ..default()
        };
        let transform = Transform::from_rotation(Quat::from_rotation_z(90f32.to_radians()));
        assert!(item_overlaps_rect(
            Vec2::new(-2.0, 40.0),
            Vec2::new(2.0, 45.0),
            &transform,
            &bar,
            &images
        ));
        assert!(!item_overlaps_rect(
            Vec2::new(20.0, -2.0),
            Vec2::new(30.0, 2.0),
            &transform,
            &bar,
            &images
        ));
    }
}
