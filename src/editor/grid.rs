use bevy::prelude::*;

use crate::map::MapData;

use super::camera::CameraZoom;
use super::EditorCamera;

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

pub fn draw_grid(
    mut gizmos: Gizmos,
    map_data: Res<MapData>,
    camera_query: Query<(&Transform, &CameraZoom), With<EditorCamera>>,
) {
    if !map_data.grid_visible {
        return;
    }

    let Ok((camera_transform, zoom)) = camera_query.single() else {
        return;
    };

    let grid_size = map_data.grid_size;
    let grid_color = Color::srgba(0.5, 0.5, 0.5, 0.3);

    let view_width = 1600.0 * zoom.scale;
    let view_height = 900.0 * zoom.scale;

    let camera_pos = camera_transform.translation.truncate();

    let start_x = ((camera_pos.x - view_width / 2.0) / grid_size).floor() as i32;
    let end_x = ((camera_pos.x + view_width / 2.0) / grid_size).ceil() as i32;
    let start_y = ((camera_pos.y - view_height / 2.0) / grid_size).floor() as i32;
    let end_y = ((camera_pos.y + view_height / 2.0) / grid_size).ceil() as i32;

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
