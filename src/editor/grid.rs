use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;

use super::params::CameraWithProjection;
use crate::map::{MapData, render_layer};
use crate::session::PlayerMapView;
use crate::theme;

#[derive(Resource)]
pub struct GridSettings {
    pub snap_enabled: bool,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self { snap_enabled: true }
    }
}

/// Snap position to the center of a grid cell (not to grid intersections)
pub fn snap_to_grid(position: Vec2, grid_size: f32, snap_enabled: bool) -> Vec2 {
    if !snap_enabled {
        return position;
    }

    let half = grid_size / 2.0;
    Vec2::new(
        (position.x / grid_size).floor() * grid_size + half,
        (position.y / grid_size).floor() * grid_size + half,
    )
}

/// Gizmo group for the grid in the player view (player-only render layer 2)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct PlayerGridGizmoGroup;

/// Keep default gizmos (the editor grid, missing-asset markers) out of the
/// player view; players get their own grid, drawn over what they can see
pub fn configure_grid_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (default_config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
    default_config.render_layers = RenderLayers::layer(render_layer::EDITOR);

    let (player_config, _) = config_store.config_mut::<PlayerGridGizmoGroup>();
    player_config.render_layers = RenderLayers::layer(render_layer::PLAYER);
}

/// Draw grid lines covering the world-space rectangle from `min` to `max`
fn draw_grid_lines<Config: GizmoConfigGroup>(
    gizmos: &mut Gizmos<Config>,
    min: Vec2,
    max: Vec2,
    grid_size: f32,
) {
    let grid_color = theme::GRID_COLOR;

    let start_x = (min.x / grid_size).floor() as i32;
    let end_x = (max.x / grid_size).ceil() as i32;
    let start_y = (min.y / grid_size).floor() as i32;
    let end_y = (max.y / grid_size).ceil() as i32;

    for x in start_x..=end_x {
        let x_pos = x as f32 * grid_size;
        gizmos.line_2d(
            Vec2::new(x_pos, start_y as f32 * grid_size),
            Vec2::new(x_pos, end_y as f32 * grid_size),
            grid_color,
        );
    }

    for y in start_y..=end_y {
        let y_pos = y as f32 * grid_size;
        gizmos.line_2d(
            Vec2::new(start_x as f32 * grid_size, y_pos),
            Vec2::new(end_x as f32 * grid_size, y_pos),
            grid_color,
        );
    }
}

pub fn draw_grid(mut gizmos: Gizmos, map_data: Res<MapData>, camera: CameraWithProjection) {
    if !map_data.grid_visible {
        return;
    }
    // Over the whole window, whatever its size
    let Some((min, max)) = camera.visible_world_rect() else {
        return;
    };
    draw_grid_lines(&mut gizmos, min, max, map_data.grid_size);
}

/// Draw the grid in the player view, over the area the players can see
pub fn draw_player_grid(mut gizmos: Gizmos<PlayerGridGizmoGroup>, player_view: PlayerMapView) {
    let map_data = player_view.map_data();
    if !map_data.player_grid_visible {
        return;
    }

    let (min, max) = player_view.viewport.world_bounds();
    draw_grid_lines(&mut gizmos, min, max, map_data.grid_size);
}

#[cfg(test)]
mod tests {
    use super::*;

    // GridSettings tests
    #[test]
    fn test_grid_settings_default() {
        let settings = GridSettings::default();
        assert!(settings.snap_enabled);
    }

    // snap_to_grid tests
    #[test]
    fn test_snap_disabled_returns_original() {
        let pos = Vec2::new(33.0, 47.0);
        let result = snap_to_grid(pos, 70.0, false);
        assert_eq!(result, pos);
    }

    #[test]
    fn test_snap_to_grid_center_of_cell() {
        // With grid_size 70, cell centers are at 35, 105, 175, etc.
        let pos = Vec2::new(10.0, 10.0);
        let result = snap_to_grid(pos, 70.0, true);
        assert_eq!(result, Vec2::new(35.0, 35.0));
    }

    #[test]
    fn test_snap_at_origin() {
        let pos = Vec2::new(0.0, 0.0);
        let result = snap_to_grid(pos, 70.0, true);
        assert_eq!(result, Vec2::new(35.0, 35.0));
    }

    #[test]
    fn test_snap_already_at_center() {
        let pos = Vec2::new(35.0, 35.0);
        let result = snap_to_grid(pos, 70.0, true);
        assert_eq!(result, Vec2::new(35.0, 35.0));
    }

    #[test]
    fn test_snap_edge_of_cell() {
        // Position at the edge (70, 70) should snap to next cell center (105, 105)
        let pos = Vec2::new(70.0, 70.0);
        let result = snap_to_grid(pos, 70.0, true);
        assert_eq!(result, Vec2::new(105.0, 105.0));
    }

    #[test]
    fn test_snap_negative_coordinates() {
        // Negative positions should also snap correctly
        let pos = Vec2::new(-10.0, -10.0);
        let result = snap_to_grid(pos, 70.0, true);
        assert_eq!(result, Vec2::new(-35.0, -35.0));
    }

    #[test]
    fn test_snap_large_negative() {
        let pos = Vec2::new(-100.0, -100.0);
        let result = snap_to_grid(pos, 70.0, true);
        assert_eq!(result, Vec2::new(-105.0, -105.0));
    }

    #[test]
    fn test_snap_different_grid_size() {
        // With grid_size 100, centers are at 50, 150, 250, etc.
        let pos = Vec2::new(75.0, 75.0);
        let result = snap_to_grid(pos, 100.0, true);
        assert_eq!(result, Vec2::new(50.0, 50.0));
    }

    #[test]
    fn test_snap_small_grid() {
        // With grid_size 10, centers are at 5, 15, 25, etc.
        let pos = Vec2::new(17.0, 22.0);
        let result = snap_to_grid(pos, 10.0, true);
        assert_eq!(result, Vec2::new(15.0, 25.0));
    }

    #[test]
    fn test_snap_asymmetric_position() {
        // Test with different X and Y cell positions
        let pos = Vec2::new(80.0, 150.0);
        let result = snap_to_grid(pos, 70.0, true);
        assert_eq!(result, Vec2::new(105.0, 175.0));
    }

    #[test]
    fn test_snap_preserves_cell() {
        // Multiple positions within the same cell should snap to the same center
        let grid_size = 70.0;
        let center = Vec2::new(35.0, 35.0);

        let positions = [
            Vec2::new(1.0, 1.0),
            Vec2::new(35.0, 35.0),
            Vec2::new(69.0, 69.0),
            Vec2::new(0.0, 69.0),
            Vec2::new(69.0, 0.0),
        ];

        for pos in positions {
            let result = snap_to_grid(pos, grid_size, true);
            assert_eq!(
                result, center,
                "Position {:?} should snap to {:?}",
                pos, center
            );
        }
    }
}
