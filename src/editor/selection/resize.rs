//! Resize math for dragging a selected item's handles.
//!
//! Everything is in an item's own frame (before its rotation): `half_size` is half its
//! size there, and positions are relative to its center.

use bevy::prelude::*;

use super::SelectionDragMode;

/// Smallest an item can be resized to, as a fraction of its size when the drag started
const MIN_RESIZE_FACTOR: f32 = 0.01;

/// Which edges a resize handle moves, per axis: 1 is the east or north edge, -1 the west
/// or south edge, 0 neither. Zero for modes that aren't resizes.
pub fn handle_edges(mode: SelectionDragMode) -> Vec2 {
    match mode {
        SelectionDragMode::ResizeE => Vec2::new(1.0, 0.0),
        SelectionDragMode::ResizeW => Vec2::new(-1.0, 0.0),
        SelectionDragMode::ResizeN => Vec2::new(0.0, 1.0),
        SelectionDragMode::ResizeS => Vec2::new(0.0, -1.0),
        SelectionDragMode::ResizeNE => Vec2::new(1.0, 1.0),
        SelectionDragMode::ResizeNW => Vec2::new(-1.0, 1.0),
        SelectionDragMode::ResizeSE => Vec2::new(1.0, -1.0),
        SelectionDragMode::ResizeSW => Vec2::new(-1.0, -1.0),
        _ => Vec2::ZERO,
    }
}

/// Where the handle for `mode` sits on an item with `half_size`.
pub fn handle_position(mode: SelectionDragMode, half_size: Vec2) -> Vec2 {
    handle_edges(mode) * half_size
}

/// How much to scale an item, per axis, when its `mode` handle is dragged to
/// `local_mouse` with the opposite edges held still. With `keep_aspect` both axes get the
/// same factor: the moving axis's for an edge handle, the larger one for a corner.
pub fn resize_factors(
    mode: SelectionDragMode,
    half_size: Vec2,
    local_mouse: Vec2,
    keep_aspect: bool,
) -> Vec2 {
    let edges = handle_edges(mode);
    let axis = |edge: f32, half: f32, mouse: f32| {
        if edge == 0.0 || half < 0.001 {
            1.0
        } else {
            // Distance from the fixed edge to the mouse, over the original size
            ((edge * mouse + half) / (2.0 * half)).max(MIN_RESIZE_FACTOR)
        }
    };
    let factors = Vec2::new(
        axis(edges.x, half_size.x, local_mouse.x),
        axis(edges.y, half_size.y, local_mouse.y),
    );
    if !keep_aspect {
        return factors;
    }
    let uniform = if edges.y == 0.0 {
        factors.x
    } else if edges.x == 0.0 {
        factors.y
    } else {
        factors.x.max(factors.y)
    };
    Vec2::splat(uniform)
}

/// How far an item's center moves when it's scaled by `factors` with the edges opposite
/// the `mode` handle held still.
pub fn resize_center_offset(mode: SelectionDragMode, half_size: Vec2, factors: Vec2) -> Vec2 {
    handle_edges(mode) * half_size * (factors - Vec2::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF: Vec2 = Vec2::new(35.0, 20.0);

    #[test]
    fn test_east_handle_moves_only_the_east_edge() {
        // Drag the east edge from x=35 to x=105: twice as wide, west edge still at -35
        let factors = resize_factors(
            SelectionDragMode::ResizeE,
            HALF,
            Vec2::new(105.0, 0.0),
            false,
        );
        assert_eq!(factors, Vec2::new(2.0, 1.0));
        let offset = resize_center_offset(SelectionDragMode::ResizeE, HALF, factors);
        assert_eq!(offset, Vec2::new(35.0, 0.0));
        assert_eq!(offset.x - HALF.x * factors.x, -HALF.x);
    }

    #[test]
    fn test_south_west_corner_keeps_north_east_corner() {
        let factors = resize_factors(
            SelectionDragMode::ResizeSW,
            HALF,
            Vec2::new(-70.0, -60.0),
            false,
        );
        assert_eq!(factors, Vec2::new(1.5, 2.0));
        let offset = resize_center_offset(SelectionDragMode::ResizeSW, HALF, factors);
        // North-east corner stays put
        assert_eq!(offset + HALF * factors, HALF);
    }

    #[test]
    fn test_keep_aspect_uses_the_dragged_axis_or_the_larger_factor() {
        let edge = resize_factors(SelectionDragMode::ResizeN, HALF, Vec2::new(0.0, 60.0), true);
        assert_eq!(edge, Vec2::splat(2.0));
        let corner = resize_factors(
            SelectionDragMode::ResizeNE,
            HALF,
            Vec2::new(70.0, 30.0),
            true,
        );
        assert_eq!(corner, Vec2::splat(1.5));
    }

    #[test]
    fn test_dragging_past_the_fixed_edge_stops_at_the_minimum() {
        let factors = resize_factors(
            SelectionDragMode::ResizeE,
            HALF,
            Vec2::new(-500.0, 0.0),
            false,
        );
        assert_eq!(factors.x, MIN_RESIZE_FACTOR);
    }

    #[test]
    fn test_handle_position_matches_edges() {
        assert_eq!(
            handle_position(SelectionDragMode::ResizeNW, HALF),
            Vec2::new(-35.0, 20.0)
        );
        assert_eq!(handle_position(SelectionDragMode::Move, HALF), Vec2::ZERO);
    }
}
