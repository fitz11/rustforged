//! Line tool system for drawing straight lines.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map::{Layer, MapData};

use super::super::params::{is_cursor_over_ui, CameraParams};
use super::super::tools::{CurrentTool, EditorTool};
use super::components::{AnnotationMarker, DrawnLine};
use super::layer_helpers::is_annotation_layer_locked;
use super::state::{AnnotationSettings, LineDrawState};

#[allow(clippy::too_many_arguments)]
pub fn handle_line(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    current_tool: Res<CurrentTool>,
    mut line_state: ResMut<LineDrawState>,
    settings: Res<AnnotationSettings>,
    map_data: Res<MapData>,
    camera: CameraParams,
    mut contexts: EguiContexts,
) {
    if current_tool.tool != EditorTool::Line {
        line_state.start_point = None;
        return;
    }

    // Don't allow line drawing if annotation layer is locked
    if is_annotation_layer_locked(&map_data) {
        return;
    }

    if is_cursor_over_ui(&mut contexts) {
        return;
    }

    let Some(world_pos) = camera.cursor_world_pos() else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(start) = line_state.start_point {
            // Second click - create line
            let z = Layer::Annotation.z_base();
            commands.spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                DrawnLine {
                    start,
                    end: world_pos,
                    color: settings.stroke_color,
                    stroke_width: settings.stroke_width,
                },
                AnnotationMarker,
            ));
            line_state.start_point = None;
        } else {
            // First click - set start point
            line_state.start_point = Some(world_pos);
        }
    }

    // Right click cancels
    if mouse_button.just_pressed(MouseButton::Right) {
        line_state.start_point = None;
    }
}
