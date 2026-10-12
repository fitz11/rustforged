pub mod annotations;
mod brush;
mod camera;
mod clipboard;
pub mod conditions;
pub mod fog;
mod grid;
pub mod history;
pub mod params;
mod placement;
mod selection;
pub mod tools;

pub use annotations::{AnnotationMarker, AnnotationSettings, DrawnLine, DrawnPath, TextAnnotation};
pub use camera::EditorCamera;
pub use clipboard::Clipboard;
pub use conditions::{
    command_key, no_dialog_open, no_text_input, plain_key, session_is_active, shortcuts_enabled,
    tool_is,
};
pub use grid::GridSettings;
pub use tools::{CurrentTool, EditorTool, SelectedLayer};

use bevy::input::common_conditions::input_pressed;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
// EguiPrimaryContextPass import removed - text tool disabled

use crate::map::{MapData, PlacedItem};

/// Update sprite visibility based on layer visibility settings. Covers every item when the
/// layer settings change, and otherwise items that were just spawned or moved to a layer.
fn update_layer_visibility(
    map_data: Res<MapData>,
    mut items_query: Query<(Ref<PlacedItem>, &mut Visibility)>,
) {
    let all = map_data.is_changed();
    for (item, mut visibility) in items_query.iter_mut() {
        if !all && !item.is_changed() {
            continue;
        }
        let layer_visible = map_data.is_layer_visible(item.layer);

        let new_visibility = if layer_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        if *visibility != new_visibility {
            *visibility = new_visibility;
        }
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<tools::CurrentTool>()
            .init_resource::<tools::SelectedLayer>()
            .init_resource::<GridSettings>()
            .init_resource::<selection::DragState>()
            .init_resource::<selection::BoxSelectState>()
            .init_resource::<annotations::DrawState>()
            .init_resource::<annotations::LineDrawState>()
            .init_resource::<annotations::TextEditState>()
            .init_resource::<annotations::AnnotationSettings>()
            .init_resource::<clipboard::Clipboard>()
            .init_resource::<history::CommandHistory>()
            .add_message::<history::RecordEditorCommand>()
            .init_resource::<fog::FogState>()
            .init_resource::<brush::BrushState>()
            // Register gizmo groups for editor-only rendering
            .init_gizmo_group::<annotations::AnnotationGizmoGroup>()
            .init_gizmo_group::<fog::FogEditorGizmoGroup>()
            .init_gizmo_group::<fog::FogPlayerGizmoGroup>()
            .init_gizmo_group::<selection::SelectionGizmoGroup>()
            .init_gizmo_group::<grid::PlayerGridGizmoGroup>()
            .add_systems(
                Startup,
                (
                    camera::spawn_camera,
                    annotations::configure_annotation_gizmos,
                    fog::configure_fog_gizmos,
                    selection::configure_selection_gizmos,
                    placement::spawn_placement_ghost,
                    grid::configure_grid_gizmos,
                ),
            )
            .add_systems(
                Update,
                (
                    camera::camera_pan.run_if(input_pressed(MouseButton::Middle)),
                    camera::camera_zoom.run_if(on_message::<MouseWheel>),
                    camera::apply_camera_zoom,
                    grid::draw_grid,
                    grid::draw_player_grid.run_if(session_is_active),
                    tools::handle_tool_shortcuts.run_if(shortcuts_enabled),
                    tools::update_cursor_icon,
                    // After handle_selection: a placement can switch to Select,
                    // and the placing click mustn't also land as a Select click
                    placement::handle_placement
                        .run_if(tool_is(EditorTool::Place).and_then(no_dialog_open))
                        .after(selection::handle_selection),
                    placement::update_placement_ghost,
                    brush::handle_brush
                        .run_if(tool_is(EditorTool::Brush).and_then(no_dialog_open)),
                    update_layer_visibility,
                ),
            )
            .add_systems(
                Update,
                (
                    selection::handle_selection
                        .run_if(tool_is(EditorTool::Select).and_then(no_dialog_open)),
                    selection::handle_box_select
                        .run_if(tool_is(EditorTool::Select).and_then(no_dialog_open)),
                    selection::handle_drag
                        .run_if(tool_is(EditorTool::Select).and_then(no_dialog_open)),
                    selection::draw_selection_indicators,
                    selection::draw_box_select_rect,
                    selection::handle_fit_to_grid.run_if(
                        plain_key(KeyCode::KeyG)
                            .and_then(tool_is(EditorTool::Select))
                            .and_then(shortcuts_enabled),
                    ),
                    selection::handle_center_to_grid.run_if(
                        plain_key(KeyCode::KeyG)
                            .and_then(tool_is(EditorTool::Select))
                            .and_then(shortcuts_enabled),
                    ),
                    selection::handle_restore_aspect_ratio.run_if(
                        plain_key(KeyCode::KeyA)
                            .and_then(tool_is(EditorTool::Select))
                            .and_then(shortcuts_enabled),
                    ),
                    selection::handle_rotate_90.run_if(
                        plain_key(KeyCode::KeyR)
                            .and_then(tool_is(EditorTool::Select))
                            .and_then(shortcuts_enabled),
                    ),
                    selection::handle_deletion.run_if(
                        plain_key(KeyCode::Delete)
                            .or_else(plain_key(KeyCode::Backspace))
                            .and_then(shortcuts_enabled),
                    ),
                    selection::handle_escape_clear_selection
                        .run_if(plain_key(KeyCode::Escape).and_then(shortcuts_enabled)),
                    selection::update_selection_cursor.run_if(tool_is(EditorTool::Select)),
                    clipboard::handle_copy
                        .run_if(command_key(KeyCode::KeyC).and_then(shortcuts_enabled)),
                    clipboard::handle_cut
                        .run_if(command_key(KeyCode::KeyX).and_then(shortcuts_enabled)),
                    clipboard::handle_paste
                        .run_if(command_key(KeyCode::KeyV).and_then(shortcuts_enabled)),
                    history::handle_undo_redo.run_if(shortcuts_enabled),
                    history::record_commands,
                ),
            )
            .add_systems(
                Update,
                (
                    annotations::handle_draw
                        .run_if(tool_is(EditorTool::Draw).and_then(no_dialog_open)),
                    annotations::handle_line
                        .run_if(tool_is(EditorTool::Line).and_then(no_dialog_open)),
                    // Text tool disabled - see TODO in tools.rs
                    // annotations::handle_text
                    //     .run_if(tool_is(EditorTool::Text).and_then(no_dialog_open)),
                    annotations::render_drawn_paths,
                    annotations::render_drawn_lines,
                    annotations::render_line_preview.run_if(tool_is(EditorTool::Line)),
                    annotations::render_draw_preview.run_if(tool_is(EditorTool::Draw)),
                ),
            )
            .add_systems(
                Update,
                (
                    fog::handle_fog.run_if(tool_is(EditorTool::Fog).and_then(no_dialog_open)),
                    fog::render_fog_editor,
                    fog::render_fog_player,
                    fog::render_fog_brush_preview.run_if(tool_is(EditorTool::Fog)),
                ),
            )
            // Text annotation systems disabled - see TODO in tools.rs
            // .add_systems(
            //     EguiPrimaryContextPass,
            //     (
            //         annotations::text_annotation_input_ui,
            //         annotations::render_text_annotations,
            //     ),
            // );
            ;
    }
}
