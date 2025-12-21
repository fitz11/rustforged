use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::map::{Layer, MapData};

pub fn layers_panel_ui(mut contexts: EguiContexts, mut map_data: ResMut<MapData>) -> Result {
    egui::SidePanel::right("layers_panel")
        .default_width(180.0)
        .show(contexts.ctx_mut()?, |ui| {
            ui.heading("Layers");

            ui.separator();

            for layer in Layer::all().iter().rev() {
                if let Some(layer_data) = map_data
                    .layers
                    .iter_mut()
                    .find(|l| l.layer_type == *layer)
                {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut layer_data.visible, "");
                        ui.label(layer.display_name());

                        if layer_data.locked {
                            if ui.small_button("🔒").clicked() {
                                layer_data.locked = false;
                            }
                        } else if ui.small_button("🔓").clicked() {
                            layer_data.locked = true;
                        }
                    });
                }
            }
        });
    Ok(())
}
