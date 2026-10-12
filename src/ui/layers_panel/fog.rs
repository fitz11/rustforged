//! Fog of War controls UI.

use bevy::prelude::Vec2;
use bevy_egui::egui;

use crate::editor::fog::cells_in_rect;
use crate::editor::history::EditorCommand;
use crate::map::{FogOfWarData, Layer, MapData};

/// How long Reset Fog waits for its confirming second click, in seconds
const RESET_CONFIRM_SECONDS: f64 = 3.0;

/// What the fog controls changed this frame.
#[derive(Default)]
pub struct FogControlsChange {
    /// The fog layer was turned on or off
    pub map: bool,
    /// The revealed cells changed, as this undo step
    pub fog: Option<EditorCommand>,
}

/// Renders the Fog of War controls section. `map_bounds` is the area the map's items
/// cover, for Reveal All.
pub fn render_fog_controls(
    ui: &mut egui::Ui,
    map_data: &mut MapData,
    fog_data: &mut FogOfWarData,
    map_bounds: Option<(Vec2, Vec2)>,
) -> FogControlsChange {
    let mut change = FogControlsChange::default();
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
    let mut fog_enabled = map_data.is_layer_visible(Layer::FogOfWar);

    if ui
        .checkbox(&mut fog_enabled, "Enable Fog of War")
        .on_hover_text("Toggle fog visibility for players")
        .changed()
    {
        if let Some(layer_data) = map_data.layer_mut(Layer::FogOfWar) {
            layer_data.visible = fog_enabled;
        }
        change.map = true;
    }

    ui.add_space(4.0);

    // Reveal All: every cell the map's items cover
    if ui
        .add_enabled(
            map_bounds.is_some(),
            egui::Button::new("Reveal All").min_size(egui::vec2(160.0, 24.0)),
        )
        .on_hover_text("Reveal everything the map's items cover (undo with Ctrl+Z)")
        .clicked()
        && let Some((min, max)) = map_bounds
    {
        let revealed: Vec<_> = cells_in_rect(min, max, map_data.grid_size)
            .filter(|cell| !fog_data.is_cell_revealed(*cell))
            .collect();
        for cell in &revealed {
            fog_data.reveal_cell(*cell);
        }
        change.fog = EditorCommand::fog(revealed, Vec::new());
    }

    ui.add_space(4.0);

    // Reset Fog covers everything again. It asks for a second click (it's undoable too).
    let confirm_id = egui::Id::new("reset_fog_armed_until");
    let now = ui.input(|i| i.time);
    let armed = ui
        .data(|d| d.get_temp::<f64>(confirm_id))
        .is_some_and(|until| now < until);
    let label = if armed {
        "Click again to reset"
    } else {
        "Reset Fog"
    };
    if ui
        .add_enabled(
            fog_data.has_revealed_cells(),
            egui::Button::new(label).min_size(egui::vec2(160.0, 24.0)),
        )
        .on_hover_text("Cover everything with fog again (undo with Ctrl+Z)")
        .clicked()
    {
        if armed {
            let hidden: Vec<_> = fog_data.revealed_cells.iter().copied().collect();
            fog_data.reset();
            change.fog = EditorCommand::fog(Vec::new(), hidden);
            ui.data_mut(|d| d.remove::<f64>(confirm_id));
        } else {
            ui.data_mut(|d| d.insert_temp(confirm_id, now + RESET_CONFIRM_SECONDS));
        }
    }

    ui.add_space(12.0);
    ui.separator();
    change
}
