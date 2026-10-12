//! Main asset browser panel UI.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::assets::{
    AssetLibrary, MapFilesChanged, RefreshAssetLibrary, RetargetAssetRequest, SelectedAsset,
    ThumbnailCache, UpdateLibraryMetadataRequest,
};
use crate::config::{AppConfig, SetDefaultLibraryRequest};
use crate::editor::CurrentTool;
use crate::ui::panel_layout::DockedPanel;

use super::assets_section::{render_asset_list, render_folder_tree, render_selected_asset_info};
use super::dialogs::{
    handle_rename_shortcuts, render_import_error_dialog, render_move_asset_dialog,
    render_rename_asset_dialog, render_rename_library_dialog, render_rename_map_dialog,
    render_set_default_dialog, render_success_dialog, render_zip_job_dialog,
};
use super::helpers::discover_folders;
use super::library_section::render_library_section;
use super::state::{AssetBrowserState, DialogStates, MapResources};

/// Main asset browser UI system.
#[allow(clippy::too_many_arguments)]
pub fn asset_browser_ui(
    mut contexts: EguiContexts,
    mut library: ResMut<AssetLibrary>,
    mut selected_asset: ResMut<SelectedAsset>,
    mut browser_state: ResMut<AssetBrowserState>,
    mut current_tool: ResMut<CurrentTool>,
    mut thumbnail_cache: ResMut<ThumbnailCache>,
    config: Res<AppConfig>,
    mut set_default_events: MessageWriter<SetDefaultLibraryRequest>,
    mut rename_events: MessageWriter<RetargetAssetRequest>,
    mut library_metadata_events: MessageWriter<UpdateLibraryMetadataRequest>,
    mut refresh_events: MessageWriter<RefreshAssetLibrary>,
    mut map_file_events: MessageReader<MapFilesChanged>,
    mut map_res: MapResources,
    mut dialogs: DialogStates,
) -> Result {
    // Clear thumbnail cache and update folders if library path changed
    let current_path = library.library_path.clone();
    if browser_state.last_library_path.as_ref() != Some(&current_path) {
        thumbnail_cache.clear();
        browser_state.discovered_folders = discover_folders(&library);
        browser_state.last_library_path = Some(current_path);
    }
    // Folders may have appeared or gone after a rescan (e.g. synced in from elsewhere)
    if browser_state.last_library_generation != library.generation() {
        browser_state.last_library_generation = library.generation();
        browser_state.discovered_folders = discover_folders(&library);
    }
    // Map files changed on disk: rescan the map list
    if map_file_events.read().count() > 0 {
        browser_state.last_maps_scan_path = None;
    }

    // Handle keyboard shortcuts for rename dialogs
    handle_rename_shortcuts(
        &mut contexts,
        &mut browser_state,
        &selected_asset,
        &map_res,
        &library,
    );

    egui::Panel::left("asset_browser")
        .default_size(220.0)
        .show_docked(contexts.ctx_mut()?, |ui| {
            render_library_section(
                ui,
                &mut library,
                &mut browser_state,
                &mut dialogs,
                &mut map_res,
                &mut thumbnail_cache,
            );

            ui.separator();
            ui.add_space(4.0);

            render_folder_tree(ui, &mut browser_state);

            ui.add_space(4.0);
            ui.separator();

            render_asset_list(
                ui,
                &library,
                &mut selected_asset,
                &mut current_tool,
                &mut browser_state,
                &thumbnail_cache,
            );

            ui.separator();
            ui.add_space(4.0);

            render_selected_asset_info(ui, &selected_asset, &mut browser_state);

            // Settings button at bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                if ui
                    .add_sized([100.0, 24.0], egui::Button::new("Settings"))
                    .clicked()
                {
                    dialogs.settings_state.load_from_config(&config);
                    dialogs.settings_state.load_library_info(&library);
                    dialogs.settings_state.is_open = true;
                }
                ui.add_space(4.0);
                ui.separator();
            });
        });

    // Render all dialogs
    let ctx = contexts.ctx_mut()?;
    render_set_default_dialog(ctx, &mut browser_state, &mut set_default_events);
    render_import_error_dialog(ctx, &mut browser_state);
    render_success_dialog(ctx, &mut browser_state);
    render_zip_job_dialog(ctx, &browser_state);
    render_rename_asset_dialog(
        ctx,
        &mut browser_state,
        &mut selected_asset,
        &mut library,
        &mut thumbnail_cache,
        &mut rename_events,
    );
    render_rename_map_dialog(ctx, &mut browser_state, &mut map_res);
    render_rename_library_dialog(ctx, &mut browser_state, &mut library_metadata_events);
    render_move_asset_dialog(
        ctx,
        &mut browser_state,
        &mut selected_asset,
        &mut library,
        &mut thumbnail_cache,
        &mut rename_events,
    );

    // Handle library refresh request
    if browser_state.refresh_requested {
        browser_state.refresh_requested = false;
        thumbnail_cache.clear();
        browser_state.discovered_folders = discover_folders(&library);
        refresh_events.write(RefreshAssetLibrary);
    }

    Ok(())
}
