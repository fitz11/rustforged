pub mod annotations;
mod camera;
mod grid;
mod placement;
mod selection;
pub mod tools;

pub use annotations::{
    AnnotationMarker, AnnotationSettings, DrawState, DrawnLine, DrawnPath, LineDrawState,
    TextAnnotation, TextEditState,
};
pub use camera::EditorCamera;
pub use grid::GridSettings;
pub use tools::{CurrentTool, EditorTool, SelectedLayer};

use bevy::prelude::*;

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
            .add_systems(Startup, camera::spawn_camera)
            .add_systems(
                Update,
                (
                    camera::camera_pan,
                    camera::camera_zoom,
                    camera::apply_camera_zoom,
                    grid::draw_grid,
                    tools::handle_tool_shortcuts,
                    tools::update_cursor_icon,
                    placement::handle_placement,
                ),
            )
            .add_systems(
                Update,
                (
                    selection::handle_selection,
                    selection::handle_box_select,
                    selection::handle_drag,
                    selection::draw_selection_indicators,
                    selection::draw_box_select_rect,
                    selection::handle_fit_to_grid,
                    selection::handle_deletion,
                ),
            )
            .add_systems(
                Update,
                (
                    annotations::handle_draw,
                    annotations::handle_line,
                    annotations::handle_text,
                    annotations::render_drawn_paths,
                    annotations::render_drawn_lines,
                    annotations::render_line_preview,
                    annotations::render_draw_preview,
                    annotations::render_text_annotations,
                ),
            );
    }
}
