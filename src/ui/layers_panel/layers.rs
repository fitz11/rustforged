//! Layer visibility and lock controls UI.

use bevy_egui::egui;

use crate::map::{Layer, MapData};

/// Renders the layers section with visibility checkboxes and lock buttons. Returns true if
/// a layer's visibility or lock changed.
///
/// The Fog of War row's checkbox is `show_fog_in_editor`, only the GM's view: whether
/// players see fog is "Enable Fog of War" in the fog controls. (The two used to be the
/// same switch, so hiding the fog to look at the map also took it off the player screen.)
pub fn render_layers(
    ui: &mut egui::Ui,
    map_data: &mut MapData,
    show_fog_in_editor: &mut bool,
) -> bool {
    let mut changed = false;
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Layers").heading().size(18.0));
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    for layer in Layer::all().iter().rev() {
        if let Some(layer_data) = map_data.layer_mut(*layer) {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(4, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if *layer == Layer::FogOfWar {
                            ui.checkbox(show_fog_in_editor, "").on_hover_text(
                                "Show the fog here in the editor. Players' fog is \
                                 Enable Fog of War below.",
                            );
                        } else if ui.checkbox(&mut layer_data.visible, "").changed() {
                            changed = true;
                        }
                        ui.label(egui::RichText::new(layer.display_name()).size(14.0));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let lock_text = if layer_data.locked { "🔒" } else { "🔓" };
                            if ui
                                .button(egui::RichText::new(lock_text).size(14.0))
                                .clicked()
                            {
                                layer_data.locked = !layer_data.locked;
                                changed = true;
                            }
                        });
                    });
                });
        }
    }
    changed
}
