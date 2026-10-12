//! Line tool system for drawing straight lines.

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map::MapData;

use super::super::history::{EditorCommand, MapObject, RecordEditorCommand};
use super::super::params::{CameraParams, is_cursor_over_ui};
use super::super::tools::{CurrentTool, EditorTool};
use super::components::DrawnLine;
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
    mut history_writer: MessageWriter<RecordEditorCommand>,
    asset_server: Res<AssetServer>,
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
            let object = MapObject::new_line(DrawnLine {
                start,
                end: world_pos,
                color: settings.stroke_color,
                stroke_width: settings.stroke_width,
            });
            object.spawn(&mut commands, &asset_server);
            history_writer.write(RecordEditorCommand {
                command: EditorCommand::Add(vec![object]),
            });
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
