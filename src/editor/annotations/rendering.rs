//! Rendering systems for annotations (editor-only via gizmos).

use bevy::gizmos::prelude::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::egui;
use bevy_egui::EguiContexts;

use super::super::camera::EditorCamera;
use super::super::tools::{CurrentTool, EditorTool};
use super::components::{DrawnLine, DrawnPath, EditingText, TextAnnotation};
use super::gizmo::AnnotationGizmoGroup;
use super::layer_helpers::is_annotation_layer_visible;
use super::state::{AnnotationSettings, DrawState, LineDrawState};
use crate::map::MapData;

pub fn render_drawn_paths(
    mut gizmos: Gizmos<AnnotationGizmoGroup>,
    paths: Query<&DrawnPath>,
    map_data: Res<MapData>,
) {
    if !is_annotation_layer_visible(&map_data) {
        return;
    }

    for path in paths.iter() {
        if path.points.len() < 2 {
            continue;
        }

        for window in path.points.windows(2) {
            gizmos.line_2d(window[0], window[1], path.color);
        }
    }
}

pub fn render_drawn_lines(
    mut gizmos: Gizmos<AnnotationGizmoGroup>,
    lines: Query<&DrawnLine>,
    map_data: Res<MapData>,
) {
    if !is_annotation_layer_visible(&map_data) {
        return;
    }

    for line in lines.iter() {
        gizmos.line_2d(line.start, line.end, line.color);
    }
}

pub fn render_line_preview(
    mut gizmos: Gizmos<AnnotationGizmoGroup>,
    current_tool: Res<CurrentTool>,
    line_state: Res<LineDrawState>,
    settings: Res<AnnotationSettings>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
) {
    if current_tool.tool != EditorTool::Line {
        return;
    }

    let Some(start) = line_state.start_point else {
        return;
    };

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Draw preview line with lower opacity
    let preview_color = settings.stroke_color.with_alpha(0.5);
    gizmos.line_2d(start, world_pos, preview_color);
}

pub fn render_draw_preview(
    mut gizmos: Gizmos<AnnotationGizmoGroup>,
    current_tool: Res<CurrentTool>,
    draw_state: Res<DrawState>,
    settings: Res<AnnotationSettings>,
) {
    if current_tool.tool != EditorTool::Draw || !draw_state.is_drawing {
        return;
    }

    if draw_state.current_points.len() < 2 {
        return;
    }

    for window in draw_state.current_points.windows(2) {
        gizmos.line_2d(window[0], window[1], settings.stroke_color);
    }
}

// Text tool disabled - see TODO in tools.rs
#[allow(dead_code)]
/// Render text annotations using egui (editor-only, doesn't show in player view)
pub fn render_text_annotations(
    mut contexts: EguiContexts,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    texts: Query<(&Transform, &TextAnnotation), Without<EditingText>>,
    map_data: Res<MapData>,
) {
    if !is_annotation_layer_visible(&map_data) {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    for (transform, text) in texts.iter() {
        // Skip empty text
        if text.content.is_empty() {
            continue;
        }

        let world_pos = transform.translation.truncate();

        // Convert world position to screen position
        let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_pos.extend(0.0))
        else {
            continue;
        };

        // Convert Bevy color to egui color
        let srgba = text.color.to_srgba();
        let egui_color = egui::Color32::from_rgba_unmultiplied(
            (srgba.red * 255.0) as u8,
            (srgba.green * 255.0) as u8,
            (srgba.blue * 255.0) as u8,
            255,
        );

        // Render the text at the screen position using egui
        egui::Area::new(egui::Id::new(format!("text_annotation_{}", world_pos)))
            .fixed_pos(egui::pos2(screen_pos.x, screen_pos.y))
            .pivot(egui::Align2::LEFT_CENTER)
            .interactable(false)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(&text.content)
                        .color(egui_color)
                        .size(text.font_size),
                );
            });
    }
}
