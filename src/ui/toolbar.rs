use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::editor::fog::FogState;
use crate::editor::{AnnotationSettings, CurrentTool, EditorTool, SelectedLayer};
use crate::map::{Layer, MapData};
use crate::session::{LiveSessionState, MonitorSelectionDialog};

/// Main toolbar showing tools and session controls
pub fn toolbar_ui(
    mut contexts: EguiContexts,
    mut current_tool: ResMut<CurrentTool>,
    mut map_data: ResMut<MapData>,
    session_state: Res<LiveSessionState>,
    mut dialog: ResMut<MonitorSelectionDialog>,
) -> Result {
    egui::TopBottomPanel::top("main_toolbar")
        .frame(
            egui::Frame::side_top_panel(&contexts.ctx_mut()?.style())
                .inner_margin(egui::Margin::symmetric(12, 8)),
        )
        .show(contexts.ctx_mut()?, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                // Tool buttons with keyboard shortcuts
                for tool in EditorTool::all() {
                    let selected = current_tool.tool == *tool;
                    let button_text = tool_button_label(tool);

                    let button = egui::Button::new(
                        egui::RichText::new(button_text).size(14.0).strong(),
                    )
                    .min_size(egui::vec2(0.0, 28.0))
                    .selected(selected);

                    let response = ui.add(button);
                    if response.clicked() {
                        current_tool.tool = *tool;
                    }
                    response.on_hover_text(tool.display_name());
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Grid toggle
                ui.checkbox(&mut map_data.grid_visible, "Grid");

                // Right-aligned session controls
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if session_state.is_active {
                        ui.colored_label(
                            egui::Color32::from_rgb(100, 200, 100),
                            egui::RichText::new("● LIVE").strong(),
                        );
                    } else if ui
                        .add(egui::Button::new("Start Session").min_size(egui::vec2(0.0, 24.0)))
                        .clicked()
                    {
                        dialog.is_open = true;
                    }
                });
            });
        });
    Ok(())
}

/// Secondary toolbar showing settings for the active tool
pub fn tool_settings_ui(
    mut contexts: EguiContexts,
    current_tool: Res<CurrentTool>,
    mut annotation_settings: ResMut<AnnotationSettings>,
    mut selected_layer: ResMut<SelectedLayer>,
    mut fog_state: ResMut<FogState>,
) -> Result {
    // Only show settings bar for tools that have settings
    let has_settings = current_tool.tool.is_annotation_tool()
        || current_tool.tool == EditorTool::Place
        || current_tool.tool == EditorTool::Brush
        || current_tool.tool == EditorTool::Fog;
    if !has_settings {
        return Ok(());
    }

    egui::TopBottomPanel::top("tool_settings")
        .frame(
            egui::Frame::side_top_panel(&contexts.ctx_mut()?.style())
                .inner_margin(egui::Margin::symmetric(12, 6))
                .fill(egui::Color32::from_rgb(45, 45, 48)),
        )
        .show(contexts.ctx_mut()?, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                match current_tool.tool {
                    EditorTool::Place | EditorTool::Brush => {
                        // Place/Brush tool settings
                        let tool_name = if current_tool.tool == EditorTool::Place {
                            "Place"
                        } else {
                            "Brush"
                        };
                        ui.label(
                            egui::RichText::new(format!("{} Settings:", tool_name))
                                .color(egui::Color32::LIGHT_GRAY),
                        );

                        ui.add_space(8.0);

                        // Layer selector
                        ui.label("Layer:");
                        let combo_id = if current_tool.tool == EditorTool::Place {
                            "place_layer_select"
                        } else {
                            "brush_layer_select"
                        };
                        egui::ComboBox::from_id_salt(combo_id)
                            .selected_text(selected_layer.layer.display_name())
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                for layer in Layer::all() {
                                    let is_selected = selected_layer.layer == *layer;
                                    if ui
                                        .selectable_label(is_selected, layer.display_name())
                                        .clicked()
                                    {
                                        selected_layer.layer = *layer;
                                    }
                                }
                            });

                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("C/Shift+C: Cycle layers")
                                .color(egui::Color32::GRAY)
                                .size(11.0),
                        );
                    }
                    EditorTool::Draw | EditorTool::Line | EditorTool::Text => {
                        // Annotation tool settings
                        let tool_name = match current_tool.tool {
                            EditorTool::Draw => "Draw",
                            EditorTool::Line => "Line",
                            EditorTool::Text => "Text",
                            _ => "",
                        };
                        ui.label(
                            egui::RichText::new(format!("{} Settings:", tool_name))
                                .color(egui::Color32::LIGHT_GRAY),
                        );

                        ui.add_space(8.0);

                        // Color selection
                        ui.label("Color:");

                        let colors = [
                            (Color::srgb(1.0, 0.0, 0.0), "Red", egui::Color32::RED),
                            (Color::srgb(0.0, 0.0, 1.0), "Blue", egui::Color32::BLUE),
                            (
                                Color::srgb(0.0, 0.8, 0.0),
                                "Green",
                                egui::Color32::from_rgb(0, 200, 0),
                            ),
                            (
                                Color::srgb(1.0, 1.0, 0.0),
                                "Yellow",
                                egui::Color32::YELLOW,
                            ),
                            (Color::srgb(0.0, 0.0, 0.0), "Black", egui::Color32::BLACK),
                            (Color::srgb(1.0, 1.0, 1.0), "White", egui::Color32::WHITE),
                            (
                                Color::srgb(1.0, 0.5, 0.0),
                                "Orange",
                                egui::Color32::from_rgb(255, 128, 0),
                            ),
                            (
                                Color::srgb(0.5, 0.0, 0.5),
                                "Purple",
                                egui::Color32::from_rgb(128, 0, 128),
                            ),
                        ];

                        for (color, name, egui_color) in colors {
                            let current_srgba = annotation_settings.stroke_color.to_srgba();
                            let srgba = color.to_srgba();
                            let is_selected = (current_srgba.red - srgba.red).abs() < 0.01
                                && (current_srgba.green - srgba.green).abs() < 0.01
                                && (current_srgba.blue - srgba.blue).abs() < 0.01;

                            let button = egui::Button::new("")
                                .fill(egui_color)
                                .min_size(egui::vec2(18.0, 18.0))
                                .stroke(if is_selected {
                                    egui::Stroke::new(2.0, egui::Color32::WHITE)
                                } else {
                                    egui::Stroke::new(1.0, egui::Color32::DARK_GRAY)
                                });

                            let response = ui.add(button);
                            if response.clicked() {
                                annotation_settings.stroke_color = color;
                            }
                            response.on_hover_text(name);
                        }

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        // Tool-specific settings
                        match current_tool.tool {
                            EditorTool::Draw | EditorTool::Line => {
                                ui.label("Width:");
                                ui.add(
                                    egui::DragValue::new(&mut annotation_settings.stroke_width)
                                        .range(1.0..=20.0)
                                        .speed(0.5)
                                        .suffix(" px"),
                                );
                            }
                            EditorTool::Text => {
                                ui.label("Font Size:");
                                ui.add(
                                    egui::DragValue::new(&mut annotation_settings.font_size)
                                        .range(8.0..=72.0)
                                        .speed(1.0)
                                        .suffix(" pt"),
                                );
                            }
                            _ => {}
                        }
                    }
                    EditorTool::Fog => {
                        // Fog tool settings
                        ui.label(
                            egui::RichText::new("Fog Settings:")
                                .color(egui::Color32::LIGHT_GRAY),
                        );

                        ui.add_space(8.0);

                        // Brush size slider
                        ui.label("Brush Size:");
                        ui.add(
                            egui::DragValue::new(&mut fog_state.brush_size)
                                .range(0.5..=5.0)
                                .speed(0.1)
                                .suffix(" cells"),
                        );

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        // Editor opacity slider
                        ui.label("Editor Opacity:");
                        ui.add(
                            egui::Slider::new(&mut fog_state.editor_opacity, 0.0..=1.0)
                                .fixed_decimals(2),
                        );

                        ui.add_space(8.0);

                        ui.label(
                            egui::RichText::new("Shift+Click: Single cell mode")
                                .color(egui::Color32::GRAY)
                                .size(11.0),
                        );
                    }
                    _ => {}
                }
            });
        });
    Ok(())
}

/// Get the button label for a tool (with keyboard shortcut)
fn tool_button_label(tool: &EditorTool) -> &'static str {
    match tool {
        EditorTool::Select => "Select [V]",
        EditorTool::Place => "Place [P]",
        EditorTool::Brush => "Brush [B]",
        EditorTool::Draw => "Draw [D]",
        EditorTool::Line => "Line [L]",
        EditorTool::Text => "Text [T]",
        EditorTool::Fog => "Fog [F]",
    }
}
