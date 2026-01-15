//! Fog of War controls UI.

use bevy_egui::egui;

use crate::map::{FogOfWarData, Layer, MapData, MapDirtyState};

/// Renders the Fog of War controls section.
pub fn render_fog_controls(
    ui: &mut egui::Ui,
    map_data: &mut MapData,
    fog_data: &mut FogOfWarData,
    dirty_state: &mut MapDirtyState,
) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Fog of War").size(14.0).strong());
        let revealed = fog_data.revealed_count();
        let status = if revealed == 0 {
            "fully fogged".to_string()
        } else {
            format!("{} revealed", revealed)
        };
        ui.label(egui::RichText::new(status).size(12.0).weak());
    });
    ui.add_space(4.0);

    // Enable/Disable toggle for fog layer
    let mut fog_enabled = map_data
        .layers
        .iter()
        .find(|l| l.layer_type == Layer::FogOfWar)
        .map(|l| l.visible)
        .unwrap_or(true);

    if ui
        .checkbox(&mut fog_enabled, "Enable Fog of War")
        .on_hover_text("Toggle fog visibility for players")
        .changed()
    {
        if let Some(layer_data) = map_data
            .layers
            .iter_mut()
            .find(|l| l.layer_type == Layer::FogOfWar)
        {
            layer_data.visible = fog_enabled;
        }
        dirty_state.is_dirty = true;
    }

    ui.add_space(4.0);

    // Reset Fog button - simply clears all revealed cells
    let reset_enabled = fog_data.has_revealed_cells();
    if ui
        .add_enabled(
            reset_enabled,
            egui::Button::new("Reset Fog").min_size(egui::vec2(160.0, 24.0)),
        )
        .on_hover_text("Hide all revealed areas (cover everything with fog)")
        .clicked()
    {
        fog_data.reset();
        dirty_state.is_dirty = true;
    }

    ui.add_space(12.0);
    ui.separator();
}
