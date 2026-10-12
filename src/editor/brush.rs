//! Brush tool for continuous asset placement.
//!
//! The Brush tool places assets continuously while the mouse is held down,
//! placing a new asset when the cursor leaves the bounds of the last placed item.
//!
//! ## Tool Behavior
//!
//! - Default (click+drag): Places assets continuously, new placement when cursor
//!   leaves the bounds of the last placed item
//! - Shift+click: Grid-fitted placement - resizes asset to fit grid cell and centers it

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::assets::SelectedAsset;
use crate::input::shift_held;
use crate::map::{MapData, PlacedItem};

use super::grid::snap_to_grid;
use super::history::{EditorCommand, MapObject, RecordEditorCommand};
use super::params::{CameraParams, is_cursor_over_ui};
use super::tools::SelectedLayer;

/// Bounding box for a placed item
#[derive(Debug, Clone, Copy)]
pub struct PlacedBounds {
    pub center: Vec2,
    pub half_size: Vec2,
}

impl PlacedBounds {
    /// Check if a point is inside the bounds
    pub fn contains(&self, point: Vec2) -> bool {
        let min = self.center - self.half_size;
        let max = self.center + self.half_size;
        point.x >= min.x && point.x <= max.x && point.y >= min.y && point.y <= max.y
    }
}

/// Resource tracking brush tool state
#[derive(Resource, Default)]
pub struct BrushState {
    /// Whether the brush is currently active (mouse held down)
    pub is_brushing: bool,
    /// The bounds of the last placed item (to avoid placement while cursor is inside)
    pub last_placed_bounds: Option<PlacedBounds>,
    /// Items placed during the current brush stroke, accumulated so the whole
    /// stroke is recorded as a single undo step on release.
    pub stroke_items: Vec<MapObject>,
}

impl BrushState {
    /// Emit the accumulated stroke (if any) as one PlaceItems command and reset.
    fn flush_stroke(&mut self, history_writer: &mut MessageWriter<RecordEditorCommand>) {
        if !self.stroke_items.is_empty() {
            history_writer.write(RecordEditorCommand {
                command: EditorCommand::Add(std::mem::take(&mut self.stroke_items)),
            });
        }
    }
}

/// Drops a brush stroke in progress when the map is replaced, so it isn't recorded as
/// an undo step of the new map.
pub fn reset_brush_on_map_change(mut brush_state: ResMut<BrushState>) {
    *brush_state = BrushState::default();
}

/// Uniform scale that fits an image of `image_size` inside one grid cell
pub(super) fn fit_to_cell_scale(image_size: Vec2, grid_size: f32) -> Vec3 {
    if image_size.x > 0.0 && image_size.y > 0.0 {
        let uniform_scale = (grid_size / image_size.x).min(grid_size / image_size.y);
        Vec3::new(uniform_scale, uniform_scale, 1.0)
    } else {
        Vec3::ONE
    }
}

/// Handle brush tool input for continuous asset placement
#[allow(clippy::too_many_arguments)]
pub fn handle_brush(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_layer: Res<SelectedLayer>,
    selected_asset: Res<SelectedAsset>,
    map_data: Res<MapData>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    camera: CameraParams,
    mut brush_state: ResMut<BrushState>,
    mut contexts: EguiContexts,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    // Mouse release - stop brushing and record the stroke, wherever the mouse is (over a
    // panel too), so the stroke is one undo step right away
    if mouse_button.just_released(MouseButton::Left) {
        brush_state.is_brushing = false;
        brush_state.last_placed_bounds = None;
        brush_state.flush_stroke(&mut history_writer);
    }

    // Don't process if cursor is over UI
    if is_cursor_over_ui(&mut contexts) {
        return;
    }

    // Handle mouse press - start brushing
    if mouse_button.just_pressed(MouseButton::Left) {
        brush_state.flush_stroke(&mut history_writer);
        brush_state.is_brushing = true;
        brush_state.last_placed_bounds = None;
    }

    // Only proceed if actively brushing
    if !brush_state.is_brushing || !mouse_button.pressed(MouseButton::Left) {
        return;
    }

    let Some(ref asset) = selected_asset.asset else {
        return;
    };

    let Some(world_pos) = camera.cursor_world_pos() else {
        return;
    };

    let grid_size = map_data.grid_size;
    let shift_held = shift_held(&keyboard);

    // Load the texture handle to get dimensions
    let texture: Handle<Image> = asset_server.load(asset.asset_path());

    // Get image dimensions (needed for both bounds checking and scale calculation)
    let (img_width, img_height) = if let Some(image) = images.get(&texture) {
        (image.width() as f32, image.height() as f32)
    } else {
        // Image not loaded yet - use grid size as fallback for bounds
        (grid_size, grid_size)
    };

    // Calculate the scale that will be applied
    let scale = if shift_held {
        fit_to_cell_scale(Vec2::new(img_width, img_height), grid_size)
    } else {
        Vec3::ONE
    };

    // Calculate the actual size of the placed item after scaling
    let placed_size = Vec2::new(img_width * scale.x, img_height * scale.y);

    // Check if cursor is still inside the last placed item's bounds
    if let Some(bounds) = brush_state.last_placed_bounds
        && bounds.contains(world_pos)
    {
        return;
    }

    // Calculate placement position (center of grid cell)
    let final_pos = snap_to_grid(world_pos, grid_size, true);

    // Update the bounds for the newly placed item
    brush_state.last_placed_bounds = Some(PlacedBounds {
        center: final_pos,
        half_size: placed_size / 2.0,
    });

    let object = MapObject::new_item(
        PlacedItem {
            asset_path: asset.path.clone(),
            layer: selected_layer.layer,
            z_index: 0,
        },
        Transform {
            translation: final_pos.extend(0.0),
            scale,
            ..default()
        },
    );
    object.spawn(&mut commands, &asset_server);

    // Accumulate for the current stroke; recorded as one undo step on release.
    brush_state.stroke_items.push(object);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fit_to_cell_scale_uses_longer_side() {
        let scale = fit_to_cell_scale(Vec2::new(140.0, 280.0), 70.0);
        assert_eq!(scale, Vec3::new(0.25, 0.25, 1.0));
    }

    #[test]
    fn test_fit_to_cell_scale_degenerate_image() {
        assert_eq!(fit_to_cell_scale(Vec2::new(0.0, 70.0), 70.0), Vec3::ONE);
    }

    #[test]
    fn test_brush_state_default() {
        let state = BrushState::default();
        assert!(!state.is_brushing);
        assert!(state.last_placed_bounds.is_none());
    }

    #[test]
    fn test_placed_bounds_contains() {
        let bounds = PlacedBounds {
            center: Vec2::new(100.0, 100.0),
            half_size: Vec2::new(50.0, 50.0),
        };

        // Center should be inside
        assert!(bounds.contains(Vec2::new(100.0, 100.0)));

        // Corners should be inside
        assert!(bounds.contains(Vec2::new(50.0, 50.0)));
        assert!(bounds.contains(Vec2::new(150.0, 150.0)));
        assert!(bounds.contains(Vec2::new(50.0, 150.0)));
        assert!(bounds.contains(Vec2::new(150.0, 50.0)));

        // Outside should not be inside
        assert!(!bounds.contains(Vec2::new(0.0, 100.0)));
        assert!(!bounds.contains(Vec2::new(200.0, 100.0)));
        assert!(!bounds.contains(Vec2::new(100.0, 0.0)));
        assert!(!bounds.contains(Vec2::new(100.0, 200.0)));
    }
}
