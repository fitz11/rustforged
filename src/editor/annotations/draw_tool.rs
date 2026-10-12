//! Draw tool system for freehand path drawing.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::ui::panel_layout::is_pointer_over_ui;

use super::super::camera::EditorCamera;
use super::super::history::{EditorCommand, MapObject, RecordEditorCommand};
use super::super::tools::EditorTool;
use super::components::DrawnPath;
use super::layer_helpers::is_annotation_layer_locked;
use super::state::{AnnotationResources, AnnotationSettings, DrawState};

pub fn handle_draw(
    mut commands: Commands,
    mut res: AnnotationResources,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut contexts: EguiContexts,
    mut history_writer: MessageWriter<RecordEditorCommand>,
    asset_server: Res<AssetServer>,
) {
    if res.current_tool.tool != EditorTool::Draw {
        // If we were drawing but switched tools, finalize
        if res.draw_state.is_drawing && res.draw_state.current_points.len() >= 2 {
            spawn_drawn_path(
                &mut commands,
                &asset_server,
                &res.draw_state,
                &res.settings,
                &mut history_writer,
            );
        }
        res.draw_state.is_drawing = false;
        res.draw_state.current_points.clear();
        return;
    }

    // Don't allow drawing if annotation layer is locked
    if is_annotation_layer_locked(&res.map_data) {
        return;
    }

    // Finish the stroke on release wherever the mouse is, over a panel or outside the
    // window too, so it isn't left as a preview
    if res.mouse_button.just_released(MouseButton::Left) {
        if res.draw_state.is_drawing && res.draw_state.current_points.len() >= 2 {
            spawn_drawn_path(
                &mut commands,
                &asset_server,
                &res.draw_state,
                &res.settings,
                &mut history_writer,
            );
        }
        res.draw_state.is_drawing = false;
        res.draw_state.current_points.clear();
        return;
    }

    if let Ok(ctx) = contexts.ctx_mut()
        && is_pointer_over_ui(ctx)
    {
        return;
    }

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

    if res.mouse_button.just_pressed(MouseButton::Left) {
        res.draw_state.is_drawing = true;
        res.draw_state.current_points.clear();
        res.draw_state.current_points.push(world_pos);
    } else if res.mouse_button.pressed(MouseButton::Left) && res.draw_state.is_drawing {
        // Add point if it's far enough from the last one (reduces point count)
        if let Some(last) = res.draw_state.current_points.last()
            && world_pos.distance(*last) > 2.0
        {
            res.draw_state.current_points.push(world_pos);
        }
    }
}

fn spawn_drawn_path(
    commands: &mut Commands,
    asset_server: &AssetServer,
    draw_state: &DrawState,
    settings: &AnnotationSettings,
    history_writer: &mut MessageWriter<RecordEditorCommand>,
) {
    let object = MapObject::new_path(DrawnPath {
        points: draw_state.current_points.clone(),
        color: settings.stroke_color,
        stroke_width: settings.stroke_width,
    });
    object.spawn(commands, asset_server);
    history_writer.write(RecordEditorCommand {
        command: EditorCommand::Add(vec![object]),
    });
}
