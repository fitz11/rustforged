//! Layer visibility and lock controls UI.

use bevy_egui::egui;

use crate::map::{Layer, MapData, MapDirtyState};

/// Renders the layers section with visibility checkboxes and lock buttons.
pub fn render_layers(ui: &mut egui::Ui, map_data: &mut MapData, dirty_state: &mut MapDirtyState) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Layers").heading().size(18.0));
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    for layer in Layer::all().iter().rev() {
        if let Some(layer_data) = map_data
            .layers
            .iter_mut()
            .find(|l| l.layer_type == *layer)
        {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(4, 4))
                .show(ui, |ui| {
                    let is_available = layer.is_available();

                    ui.add_enabled_ui(is_available, |ui| {
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut layer_data.visible, "").changed() {
                                dirty_state.is_dirty = true;
                            }
                            ui.label(egui::RichText::new(layer.display_name()).size(14.0));

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if !is_available {
                                        ui.label(
                                            egui::RichText::new("Soon")
                                                .size(10.0)
                                                .weak()
                                                .italics(),
                                        );
                                    } else {
                                        let lock_text =
                                            if layer_data.locked { "🔒" } else { "🔓" };
                                        if ui
                                            .button(egui::RichText::new(lock_text).size(14.0))
                                            .clicked()
                                        {
                                            layer_data.locked = !layer_data.locked;
                                            dirty_state.is_dirty = true;
                                        }
                                    }
                                },
                            );
                        });
                    });
                });
        }
    }
}
