//! Main layers panel UI orchestration.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::assets::{RetargetAssetRequest, SelectedAsset};
use crate::config::AppConfig;
use crate::map::{FogOfWarData, MapData, MapDirtyState, MissingAsset, Selected};
use crate::session::LiveSessionState;
use crate::ui::panel_layout::DockedPanel;

use super::HelpWindowState;
use super::fog::render_fog_controls;
use super::layers::render_layers;
use super::properties::{SelectedQuery, render_properties};
use super::session::render_session_controls;

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
    missing_query: Query<Entity, (With<Selected>, With<MissingAsset>)>,
    selected_asset: Res<SelectedAsset>,
    mut retarget_events: MessageWriter<RetargetAssetRequest>,
    config: Res<AppConfig>,
) -> Result {
    let missing: Vec<Entity> = missing_query.iter().collect();
    let library_selection = selected_asset.asset.as_ref().map(|a| a.path.as_str());
    let mut relink = None;

    egui::Panel::right("layers_panel")
        .default_size(200.0)
        .show_docked(contexts.ctx_mut()?, |ui| {
            // Edit through bypass_change_detection so these resources only count as changed
            // (for change detection, e.g. layer visibility) when the user changed something.
            // Layers section
            let layers_changed = render_layers(ui, map_data.bypass_change_detection());

            // Fog of War controls
            let fog_change = render_fog_controls(
                ui,
                map_data.bypass_change_detection(),
                fog_data.bypass_change_detection(),
            );
            if layers_changed || fog_change.map {
                map_data.set_changed();
                dirty_state.is_dirty = true;
            }
            if fog_change.fog {
                fog_data.set_changed();
                dirty_state.is_dirty = true;
            }

            // Properties section
            relink = render_properties(
                ui,
                &map_data,
                &mut selected_query,
                &images,
                &missing,
                library_selection,
            );

            // Live Session controls (when active)
            if render_session_controls(
                ui,
                session_state.bypass_change_detection(),
                config.data.move_player_view_while_frozen,
            ) {
                session_state.set_changed();
            }

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
    if let Some(request) = relink {
        retarget_events.write(request);
    }
    Ok(())
}
