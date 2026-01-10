pub mod annotations;
mod camera;
mod clipboard;
mod grid;
pub mod params;
mod placement;
mod selection;
pub mod tools;

pub use annotations::{
    AnnotationMarker, AnnotationSettings, DrawnLine, DrawnPath, TextAnnotation,
};
pub use camera::EditorCamera;
pub use grid::GridSettings;
pub use tools::{CurrentTool, EditorTool, SelectedLayer};

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::map::{MapData, PlacedItem};

/// Update sprite visibility based on layer visibility settings
fn update_layer_visibility(
    map_data: Res<MapData>,
    mut items_query: Query<(&PlacedItem, &mut Visibility)>,
) {
    for (item, mut visibility) in items_query.iter_mut() {
        let layer_visible = map_data
            .layers
            .iter()
            .find(|ld| ld.layer_type == item.layer)
            .map(|ld| ld.visible)
            .unwrap_or(true);

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
            // Register annotation gizmo group for editor-only rendering
            .init_gizmo_group::<annotations::AnnotationGizmoGroup>()
            .add_systems(
                Startup,
                (
                    camera::spawn_camera,
                    annotations::configure_annotation_gizmos,
                ),
            )
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
                    update_layer_visibility,
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
                    selection::update_selection_cursor,
                    clipboard::handle_copy,
                    clipboard::handle_cut,
                    clipboard::handle_paste,
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
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    annotations::text_annotation_input_ui,
                    annotations::render_text_annotations,
                ),
            );
    }
}
