//! Help popup window and keyboard shortcut handling.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::update::UpdateState;

use super::HelpWindowState;

/// Renders the help popup window with keyboard shortcuts and usage instructions.
pub fn help_popup_ui(
    mut contexts: EguiContexts,
    mut help_state: ResMut<HelpWindowState>,
    update_state: Res<UpdateState>,
) -> Result {
    if !help_state.is_open {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Help")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(400.0)
        .max_height(800.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Rustforged - D&D 5E VTT Map Editor");

                ui.horizontal(|ui| {
                    ui.label("Version:");
                    ui.strong(crate::update::CURRENT_VERSION);

                    if update_state.update_available
                        && let Some(ref version) = update_state.latest_version
                    {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 165, 0),
                            format!("(v{} available)", version),
                        );
                    }
                });

                ui.separator();

                // Tools Section
                render_tools_section(ui);

                ui.add_space(10.0);
                ui.separator();

                // Selection Shortcuts
                render_selection_section(ui);

                ui.add_space(10.0);
                ui.separator();

                // Camera Controls
                render_camera_section(ui);

                ui.add_space(10.0);
                ui.separator();

                // Placement
                render_placement_section(ui);

                ui.add_space(10.0);
                ui.separator();

                // Asset Management
                render_assets_section(ui);

                ui.add_space(10.0);
                ui.separator();

                // File Operations
                render_file_section(ui);

                ui.add_space(10.0);
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("GitHub:");
                    ui.hyperlink_to("fitz11/rustforged", "https://github.com/fitz11/rustforged");
                });

                ui.add_space(10.0);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui.button("Close").clicked() {
                        help_state.is_open = false;
                    }
                });
            });
        });

    // Close on Escape key
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        help_state.is_open = false;
    }

    Ok(())
}

fn render_tools_section(ui: &mut egui::Ui) {
    ui.heading("Tools");
    egui::Grid::new("tools_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            ui.strong("V / S");
            ui.label("Select - Click to select items, drag to move");
            ui.end_row();

            ui.strong("P");
            ui.label("Place - Single-click to place selected asset");
            ui.end_row();

            ui.strong("B");
            ui.label("Brush - Drag to continuously place assets");
            ui.end_row();

            ui.strong("D");
            ui.label("Draw - Freehand annotation paths");
            ui.end_row();

            ui.strong("L");
            ui.label("Line - Straight line annotations");
            ui.end_row();

            ui.strong("F");
            ui.label("Fog - Reveal/hide fog of war areas");
            ui.end_row();

            ui.strong("C / Shift+C");
            ui.label("Cycle layer (Place/Brush tools)");
            ui.end_row();
        });
}

fn render_selection_section(ui: &mut egui::Ui) {
    ui.heading("Selection");
    egui::Grid::new("selection_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            ui.strong("Click");
            ui.label("Select topmost item");
            ui.end_row();

            ui.strong("Ctrl+Click");
            ui.label("Toggle selection (multi-select)");
            ui.end_row();

            ui.strong("Drag (empty)");
            ui.label("Box selection");
            ui.end_row();

            ui.strong("Escape");
            ui.label("Clear selection");
            ui.end_row();

            ui.strong("Delete / Backspace");
            ui.label("Delete selected items");
            ui.end_row();

            ui.strong("G");
            ui.label("Fit selected to grid cell");
            ui.end_row();

            ui.strong("Shift+G");
            ui.label("Center selected to grid");
            ui.end_row();

            ui.strong("A");
            ui.label("Restore aspect ratio");
            ui.end_row();

            ui.strong("R / Shift+R");
            ui.label("Rotate 90° CW / CCW");
            ui.end_row();

            ui.strong("Ctrl+C / Ctrl+X");
            ui.label("Copy / Cut selected items");
            ui.end_row();

            ui.strong("Ctrl+V");
            ui.label("Paste items");
            ui.end_row();
        });
}

fn render_camera_section(ui: &mut egui::Ui) {
    ui.heading("Camera");
    egui::Grid::new("camera_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            ui.strong("Middle Mouse Drag");
            ui.label("Pan camera");
            ui.end_row();

            ui.strong("Scroll Wheel");
            ui.label("Zoom in/out");
            ui.end_row();
        });
}

fn render_placement_section(ui: &mut egui::Ui) {
    ui.heading("Placement");
    egui::Grid::new("placement_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            ui.strong("Click (Place tool)");
            ui.label("Place asset at grid-snapped position");
            ui.end_row();

            ui.strong("Shift+Click");
            ui.label("Free placement (bypass grid snap)");
            ui.end_row();

            ui.strong("Shift+Drag");
            ui.label("Snap selected to grid while dragging");
            ui.end_row();
        });
}

fn render_assets_section(ui: &mut egui::Ui) {
    ui.heading("Assets");
    egui::Grid::new("asset_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            ui.strong("F2");
            ui.label("Rename selected asset");
            ui.end_row();

            ui.strong("F3");
            ui.label("Rename current map");
            ui.end_row();

            ui.strong("F4");
            ui.label("Rename library");
            ui.end_row();
        });
}

fn render_file_section(ui: &mut egui::Ui) {
    ui.heading("File");
    egui::Grid::new("file_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            ui.strong("Ctrl+S");
            ui.label("Save map");
            ui.end_row();

            ui.strong("Ctrl+Shift+S");
            ui.label("Save as...");
            ui.end_row();

            ui.strong("Ctrl+O");
            ui.label("Open map");
            ui.end_row();

            ui.strong("Ctrl+N");
            ui.label("New map");
            ui.end_row();

            ui.strong("H");
            ui.label("Toggle this help window");
            ui.end_row();
        });
}

/// Handles the H keyboard shortcut to toggle help window.
pub fn handle_help_shortcut(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut help_state: ResMut<HelpWindowState>,
    mut contexts: EguiContexts,
) {
    // Don't toggle if typing in a text field
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyH) {
        help_state.is_open = !help_state.is_open;
    }
}
