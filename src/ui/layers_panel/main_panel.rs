//! Main layers panel UI orchestration.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::assets::{RetargetAssetRequest, SelectedAsset};
use crate::config::AppConfig;
use crate::editor::SelectionEdit;
use crate::editor::history::{EditorCommand, MapObject, RecordEditorCommand};
use crate::map::{FogOfWarData, MapData, MapDirtyState, MissingAsset, Selected};
use crate::session::LiveSessionState;
use crate::ui::panel_layout::DockedPanel;

use super::HelpWindowState;
use super::fog::render_fog_controls;
use super::layers::render_layers;
use super::properties::{
    ItemEdit, OtherItemsStack, PropertiesOutput, SelectedQuery, render_properties,
};
use super::session::render_session_controls;

/// Main layers panel UI system.
#[allow(clippy::too_many_arguments)]
pub fn layers_panel_ui(
    mut contexts: EguiContexts,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut selected_query: SelectedQuery,
    mut session_state: ResMut<LiveSessionState>,
    mut help_state: ResMut<HelpWindowState>,
    missing_query: Query<Entity, (With<Selected>, With<MissingAsset>)>,
    selected_asset: Res<SelectedAsset>,
    mut retarget_events: MessageWriter<RetargetAssetRequest>,
    config: Res<AppConfig>,
    mut edit_events: (
        MessageWriter<SelectionEdit>,
        MessageWriter<RecordEditorCommand>,
    ),
    mut unfinished_edit: Local<Option<MapObject>>,
    stack: OtherItemsStack,
) -> Result {
    let missing: Vec<Entity> = missing_query.iter().collect();
    let library_selection = selected_asset.asset.as_ref().map(|a| a.path.as_str());
    let mut properties = PropertiesOutput::default();

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
            properties =
                render_properties(ui, &mut selected_query, &stack, &missing, library_selection);

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
    if let Some(request) = properties.relink {
        retarget_events.write(request);
    }
    if let Some(edit) = properties.selection_edit {
        edit_events.0.write(edit);
    }
    if let Some(command) = finished_item_edit(properties.item_edit, &mut unfinished_edit) {
        edit_events.1.write(RecordEditorCommand { command });
    }
    Ok(())
}

/// Turns Properties panel edits into undo steps: one per finished edit, so dragging a
/// number field is one step rather than one per frame. `unfinished` holds the item's state
/// from before an edit that's still in progress.
fn finished_item_edit(
    edit: Option<ItemEdit>,
    unfinished: &mut Option<MapObject>,
) -> Option<EditorCommand> {
    let Some(ItemEdit {
        before,
        after,
        editing,
    }) = edit
    else {
        // The selection changed mid-edit; what was changed so far stays without a step
        *unfinished = None;
        return None;
    };
    if unfinished
        .as_ref()
        .is_some_and(|start| start.id() != after.id())
    {
        *unfinished = None;
    }
    if before != after && unfinished.is_none() {
        *unfinished = Some(before);
    }
    if editing {
        return None;
    }
    EditorCommand::change([(unfinished.take()?, after)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Layer, PlacedItem};

    fn token_at(object: &MapObject, x: f32) -> MapObject {
        let MapObject::Item { id, item, .. } = object else {
            unreachable!()
        };
        MapObject::item(*id, item.clone(), Transform::from_xyz(x, 0.0, 0.0))
    }

    #[test]
    fn test_dragging_a_field_is_one_undo_step() {
        let start = MapObject::new_item(
            PlacedItem {
                asset_path: "token.png".to_string(),
                layer: Layer::Token,
                z_index: 0,
            },
            Transform::default(),
        );
        let mut unfinished = None;
        let frame = |before: f32, after: f32, editing| {
            Some(ItemEdit {
                before: token_at(&start, before),
                after: token_at(&start, after),
                editing,
            })
        };

        assert_eq!(
            finished_item_edit(frame(0.0, 5.0, true), &mut unfinished),
            None
        );
        assert_eq!(
            finished_item_edit(frame(5.0, 9.0, true), &mut unfinished),
            None
        );
        let command = finished_item_edit(frame(9.0, 9.0, false), &mut unfinished);
        assert_eq!(
            command,
            Some(EditorCommand::Change(vec![(
                token_at(&start, 0.0),
                token_at(&start, 9.0)
            )]))
        );
        assert!(unfinished.is_none());

        // Nothing changed: no step
        assert_eq!(
            finished_item_edit(frame(9.0, 9.0, false), &mut unfinished),
            None
        );
    }
}
