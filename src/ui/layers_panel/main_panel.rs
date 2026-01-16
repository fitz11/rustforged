//! Main layers panel UI orchestration.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::map::{FogOfWarData, MapData, MapDirtyState};
use crate::session::LiveSessionState;

use super::fog::render_fog_controls;
use super::layers::render_layers;
use super::properties::{render_properties, SelectedQuery};
use super::session::render_session_controls;
use super::HelpWindowState;

/// Main layers panel UI system.
#[allow(clippy::too_many_arguments)]
pub fn layers_panel_ui(
    mut contexts: EguiContexts,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut selected_query: SelectedQuery,
    images: Res<Assets<Image>>,
    mut session_state: ResMut<LiveSessionState>,
    mut help_state: ResMut<HelpWindowState>,
) -> Result {
    egui::SidePanel::right("layers_panel")
        .default_width(200.0)
        .show(contexts.ctx_mut()?, |ui| {
            // Layers section
            render_layers(ui, &mut map_data);

            // Fog of War controls
            render_fog_controls(ui, &mut map_data, &mut fog_data, &mut dirty_state);

            // Properties section
            render_properties(ui, &map_data, &mut selected_query, &images);

            // Live Session controls (when active)
            render_session_controls(ui, &mut session_state);

            // Help button and version (bottom center)
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                ui.weak(format!("v{}", crate::update::CURRENT_VERSION));
                ui.add_space(4.0);
                if ui
                    .add_sized([120.0, 28.0], egui::Button::new("Help (H)"))
                    .clicked()
                {
                    help_state.is_open = true;
                }
            });
        });
    Ok(())
}
