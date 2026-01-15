//! Selection gizmo drawing - visual indicators for selected items.

use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;

use crate::editor::annotations::{
    is_annotation_layer_visible, line_bounds, path_bounds, text_bounds, AnnotationMarker,
    DrawnLine, DrawnPath, TextAnnotation,
};
use crate::map::{MapData, Selected};

use super::hit_detection::{get_sprite_half_size, rotate_point};
use super::{BoxSelectState, ROTATION_HANDLE_OFFSET, ROTATION_HANDLE_RADIUS};

/// Custom gizmo group for selection indicators (editor-only rendering)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct SelectionGizmoGroup;

/// Configure the selection gizmo group to only render to editor camera
pub fn configure_selection_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<SelectionGizmoGroup>();
    // Only render to layer 1 (editor-only)
    config.render_layers = RenderLayers::layer(1);
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn draw_selection_indicators(
    mut gizmos: Gizmos<SelectionGizmoGroup>,
    selected_sprites_query: Query<(&Transform, &Sprite), With<Selected>>,
    images: Res<Assets<Image>>,
    map_data: Res<MapData>,
    // Annotation queries
    selected_paths_query: Query<&DrawnPath, (With<Selected>, With<AnnotationMarker>)>,
    selected_lines_query: Query<&DrawnLine, (With<Selected>, With<AnnotationMarker>)>,
    selected_texts_query: Query<
        (&Transform, &TextAnnotation),
        (With<Selected>, With<AnnotationMarker>),
    >,
) {
    let selection_color = Color::srgb(0.2, 0.6, 1.0);
    let annotation_visible = is_annotation_layer_visible(&map_data);

    // Draw selection for sprites (placed items)
    for (transform, sprite) in selected_sprites_query.iter() {
        let pos = transform.translation.truncate();
        let scale = transform.scale.truncate();
        let half_size = get_sprite_half_size(sprite, &images);
        let scaled_half = half_size * scale;

        // Get the rotation angle from the transform
        // EulerRot::ZYX returns (z, y, x) - we want the Z rotation (first component)
        let (angle, _, _) = transform.rotation.to_euler(EulerRot::ZYX);

        // Draw rotated selection rectangle
        gizmos.rect_2d(
            Isometry2d::new(pos, Rot2::radians(angle)),
            scaled_half * 2.0,
            selection_color,
        );

        // Draw corner handles (larger) at rotated positions
        let corner_handle_size = 4.0;
        let local_corners = [
            Vec2::new(-scaled_half.x, -scaled_half.y), // SW
            Vec2::new(scaled_half.x, -scaled_half.y),  // SE
            Vec2::new(scaled_half.x, scaled_half.y),   // NE
            Vec2::new(-scaled_half.x, scaled_half.y),  // NW
        ];

        for local_corner in local_corners {
            let world_corner = rotate_point(pos + local_corner, pos, angle);
            gizmos.rect_2d(
                Isometry2d::from_translation(world_corner),
                Vec2::splat(corner_handle_size * 2.0),
                selection_color,
            );
        }

        // Draw edge handles (smaller) at rotated positions
        let edge_handle_size = 3.0;
        let local_edges = [
            Vec2::new(0.0, -scaled_half.y), // S
            Vec2::new(0.0, scaled_half.y),  // N
            Vec2::new(-scaled_half.x, 0.0), // W
            Vec2::new(scaled_half.x, 0.0),  // E
        ];

        for local_edge in local_edges {
            let world_edge = rotate_point(pos + local_edge, pos, angle);
            gizmos.rect_2d(
                Isometry2d::from_translation(world_edge),
                Vec2::splat(edge_handle_size * 2.0),
                selection_color,
            );
        }

        // Draw rotation handle above the item, rotated with the item
        let local_top = Vec2::new(0.0, scaled_half.y);
        let local_handle = Vec2::new(0.0, scaled_half.y + ROTATION_HANDLE_OFFSET);
        let world_top = rotate_point(pos + local_top, pos, angle);
        let world_handle = rotate_point(pos + local_handle, pos, angle);

        // Draw connecting line from top edge to rotation handle
        gizmos.line_2d(world_top, world_handle, selection_color);

        // Draw circular rotation handle
        gizmos.circle_2d(
            Isometry2d::from_translation(world_handle),
            ROTATION_HANDLE_RADIUS,
            selection_color,
        );
    }

    // Only draw annotation selections if the layer is visible
    if !annotation_visible {
        return;
    }

    // Draw selection for paths
    for path in selected_paths_query.iter() {
        let (min, max) = path_bounds(path);
        let center = (min + max) / 2.0;
        let size = max - min;

        gizmos.rect_2d(Isometry2d::from_translation(center), size, selection_color);

        // Draw corner handles
        let handle_size = 4.0;
        let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];

        for corner in corners {
            gizmos.rect_2d(
                Isometry2d::from_translation(corner),
                Vec2::splat(handle_size * 2.0),
                selection_color,
            );
        }
    }

    // Draw selection for lines
    for line in selected_lines_query.iter() {
        let (min, max) = line_bounds(line);
        let center = (min + max) / 2.0;
        let size = max - min;

        // Lines might be very thin in one dimension, ensure minimum size for visibility
        let size = size.max(Vec2::splat(10.0));

        gizmos.rect_2d(Isometry2d::from_translation(center), size, selection_color);

        // Draw handles at line endpoints
        let handle_size = 4.0;
        gizmos.rect_2d(
            Isometry2d::from_translation(line.start),
            Vec2::splat(handle_size * 2.0),
            selection_color,
        );
        gizmos.rect_2d(
            Isometry2d::from_translation(line.end),
            Vec2::splat(handle_size * 2.0),
            selection_color,
        );
    }

    // Draw selection for text annotations
    for (transform, text) in selected_texts_query.iter() {
        let (min, max) = text_bounds(transform, text);
        let center = (min + max) / 2.0;
        let size = max - min;

        gizmos.rect_2d(Isometry2d::from_translation(center), size, selection_color);

        // Draw corner handles
        let handle_size = 4.0;
        let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];

        for corner in corners {
            gizmos.rect_2d(
                Isometry2d::from_translation(corner),
                Vec2::splat(handle_size * 2.0),
                selection_color,
            );
        }
    }
}

pub fn draw_box_select_rect(mut gizmos: Gizmos<SelectionGizmoGroup>, box_select_state: Res<BoxSelectState>) {
    if !box_select_state.is_selecting {
        return;
    }

    let box_color = Color::srgba(0.2, 0.6, 1.0, 0.8);
    let fill_color = Color::srgba(0.2, 0.6, 1.0, 0.1);

    let start = box_select_state.start_world;
    let current = box_select_state.current_world;

    let center = (start + current) / 2.0;
    let size = (current - start).abs();

    // Draw the selection box outline
    gizmos.rect_2d(Isometry2d::from_translation(center), size, box_color);

    // Draw a semi-transparent fill (using multiple lines for visual effect)
    // Since gizmos don't have filled rectangles, we draw dashed inner lines
    let min = start.min(current);
    let max = start.max(current);
    let step = 10.0;

    // Horizontal lines
    let mut y = min.y + step;
    while y < max.y {
        gizmos.line_2d(Vec2::new(min.x, y), Vec2::new(max.x, y), fill_color);
        y += step;
    }
}
