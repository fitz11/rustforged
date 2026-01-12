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

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::assets::SelectedAsset;
use crate::map::{MapData, PlacedItem};

use super::grid::snap_to_grid;
use super::params::{is_cursor_over_ui, CameraParams};
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
) {
    // Don't process if cursor is over UI
    if is_cursor_over_ui(&mut contexts) {
        return;
    }

    // Handle mouse press - start brushing
    if mouse_button.just_pressed(MouseButton::Left) {
        brush_state.is_brushing = true;
        brush_state.last_placed_bounds = None;
    }

    // Handle mouse release - stop brushing
    if mouse_button.just_released(MouseButton::Left) {
        brush_state.is_brushing = false;
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
    let shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    // Load the texture handle to get dimensions
    let texture: Handle<Image> = asset_server.load(&asset.relative_path);

    // Get image dimensions (needed for both bounds checking and scale calculation)
    let (img_width, img_height) = if let Some(image) = images.get(&texture) {
        (image.width() as f32, image.height() as f32)
    } else {
        // Image not loaded yet - use grid size as fallback for bounds
        (grid_size, grid_size)
    };

    // Calculate the scale that will be applied
    let scale = if shift_held {
        if img_width > 0.0 && img_height > 0.0 {
            let scale_x = grid_size / img_width;
            let scale_y = grid_size / img_height;
            let uniform_scale = scale_x.min(scale_y);
            Vec3::new(uniform_scale, uniform_scale, 1.0)
        } else {
            Vec3::ONE
        }
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

    let layer = selected_layer.layer;
    let z = layer.z_base();

    // Items on non-player-visible layers go to render layer 1 (editor-only)
    let render_layer = if layer.is_player_visible() {
        RenderLayers::layer(0)
    } else {
        RenderLayers::layer(1)
    };

    commands.spawn((
        Sprite::from_image(texture),
        Transform {
            translation: final_pos.extend(z),
            scale,
            ..default()
        },
        PlacedItem {
            asset_path: asset.relative_path.clone(),
            layer,
            z_index: 0,
        },
        render_layer,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

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
