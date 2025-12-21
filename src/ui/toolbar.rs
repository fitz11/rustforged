use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::editor::{AnnotationSettings, CurrentTool, EditorTool, SelectedLayer};
use crate::map::{Layer, MapData};
use crate::session::{LiveSessionState, MonitorSelectionDialog};

pub fn toolbar_ui(
    mut contexts: EguiContexts,
    mut current_tool: ResMut<CurrentTool>,
    mut selected_layer: ResMut<SelectedLayer>,
    mut map_data: ResMut<MapData>,
    mut annotation_settings: ResMut<AnnotationSettings>,
    session_state: Res<LiveSessionState>,
    mut dialog: ResMut<MonitorSelectionDialog>,
) -> Result {
    egui::TopBottomPanel::top("toolbar").show(contexts.ctx_mut()?, |ui| {
        ui.horizontal(|ui| {
            ui.label("Tool:");
            for tool in EditorTool::all() {
                let selected = current_tool.tool == *tool;
                if ui.selectable_label(selected, tool.display_name()).clicked() {
                    current_tool.tool = *tool;
                }
            }

            ui.separator();

            ui.label("Layer:");
            egui::ComboBox::from_id_salt("layer_select")
                .selected_text(selected_layer.layer.display_name())
                .show_ui(ui, |ui| {
                    for layer in Layer::all() {
                        let is_selected = selected_layer.layer == *layer;
                        if ui.selectable_label(is_selected, layer.display_name()).clicked() {
                            selected_layer.layer = *layer;
                        }
                    }
                });

            ui.separator();

            ui.checkbox(&mut map_data.grid_visible, "Show Grid");

            // Annotation tool settings
            if current_tool.tool.is_annotation_tool() {
                ui.separator();

                // Color presets
                ui.label("Color:");
                let colors = [
                    (Color::srgb(1.0, 0.0, 0.0), "Red"),
                    (Color::srgb(0.0, 0.0, 1.0), "Blue"),
                    (Color::srgb(0.0, 0.8, 0.0), "Green"),
                    (Color::srgb(1.0, 1.0, 0.0), "Yellow"),
                    (Color::srgb(0.0, 0.0, 0.0), "Black"),
                    (Color::srgb(1.0, 1.0, 1.0), "White"),
                ];
                for (color, name) in colors {
                    let srgba = color.to_srgba();
                    let egui_color =
                        egui::Color32::from_rgb(
                            (srgba.red * 255.0) as u8,
                            (srgba.green * 255.0) as u8,
                            (srgba.blue * 255.0) as u8,
                        );
                    let current_srgba = annotation_settings.stroke_color.to_srgba();
                    let is_selected = (current_srgba.red - srgba.red).abs() < 0.01
                        && (current_srgba.green - srgba.green).abs() < 0.01
                        && (current_srgba.blue - srgba.blue).abs() < 0.01;

                    let response = ui.add(
                        egui::Button::new("")
                            .fill(egui_color)
                            .min_size(egui::vec2(20.0, 20.0))
                            .frame(is_selected),
                    );
                    if response.clicked() {
                        annotation_settings.stroke_color = color;
                    }
                    response.on_hover_text(name);
                }

                ui.separator();

                // Stroke width or font size based on tool
                match current_tool.tool {
                    EditorTool::Draw | EditorTool::Line => {
                        ui.label("Width:");
                        ui.add(
                            egui::DragValue::new(&mut annotation_settings.stroke_width)
                                .range(1.0..=20.0)
                                .speed(0.5),
                        );
                    }
                    EditorTool::Text => {
                        ui.label("Font:");
                        ui.add(
                            egui::DragValue::new(&mut annotation_settings.font_size)
                                .range(8.0..=72.0)
                                .speed(1.0),
                        );
                    }
                    _ => {}
                }
            }

            ui.separator();

            // Live Session controls
            if session_state.is_active {
                ui.label("Session Active");
                ui.colored_label(egui::Color32::GREEN, "(Live)");
            } else if ui.button("Start Live Session").clicked() {
                dialog.is_open = true;
            }
        });
    });
    Ok(())
}
