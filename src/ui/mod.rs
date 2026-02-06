mod asset_browser;
pub mod asset_import;
pub mod file_menu;
mod layers_panel;
mod session_controls;
mod settings_dialog;
mod toolbar;


use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::config::{ConfigResetNotification, MissingMapWarning};
use crate::map::{
    AsyncMapOperation, LoadValidationWarning, MapLoadError, MapSaveError, SaveValidationWarning,
    UnsavedChangesDialog,
};
use crate::session::MonitorSelectionDialog;

/// Resource that tracks whether any modal dialog is currently open.
/// Editor input handlers should check this to avoid processing input
/// when the user is interacting with a dialog.
#[derive(Resource, Default)]
pub struct DialogState {
    /// True when any modal dialog is open that should block editor input
    pub any_modal_open: bool,
}

/// System to aggregate all dialog open states into a single resource.
/// Runs in First schedule before input handlers.
#[allow(clippy::too_many_arguments)]
fn update_dialog_state(
    file_menu: Res<file_menu::FileMenuState>,
    asset_browser: Res<asset_browser::AssetBrowserState>,
    asset_import: Res<asset_import::AssetImportDialog>,
    settings: Res<settings_dialog::SettingsDialogState>,
    help: Res<layers_panel::HelpWindowState>,
    monitor_dialog: Res<MonitorSelectionDialog>,
    missing_map: Res<MissingMapWarning>,
    config_reset: Res<ConfigResetNotification>,
    unsaved_changes: Res<UnsavedChangesDialog>,
    save_validation: Res<SaveValidationWarning>,
    load_validation: Res<LoadValidationWarning>,
    save_error: Res<MapSaveError>,
    load_error: Res<MapLoadError>,
    async_op: Res<AsyncMapOperation>,
    mut dialog_state: ResMut<DialogState>,
) {
    dialog_state.any_modal_open = file_menu.show_new_confirmation
        || file_menu.show_save_name_dialog
        || asset_browser.rename_dialog_open
        || asset_browser.rename_map_dialog_open
        || asset_browser.rename_library_dialog_open
        || asset_browser.move_dialog_open
        || asset_browser.show_set_default_dialog
        || asset_import.is_open
        || settings.is_open
        || help.is_open
        || monitor_dialog.is_open
        || missing_map.show
        || config_reset.show
        || unsaved_changes.show_close_confirmation
        || save_validation.show
        || load_validation.show
        || save_error.message.is_some()
        || load_error.message.is_some()
        || async_op.is_busy()
        || asset_browser.any_file_dialog_pending()
        || asset_import.pending_browse.is_some()
        || settings.pending_browse.is_some();
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogState>()
            .init_resource::<asset_browser::AssetBrowserState>()
            .init_resource::<asset_import::AssetImportDialog>()
            .init_resource::<file_menu::FileMenuState>()
            .init_resource::<layers_panel::HelpWindowState>()
            .init_resource::<settings_dialog::SettingsDialogState>()
            // Load thumbnails before egui pass
            .add_systems(Update, asset_browser::load_and_register_thumbnails)
            // Side panels must render first so top panels fit between them
            // Use chain() to enforce ordering
            .add_systems(
                EguiPrimaryContextPass,
                (
                    // First: side panels
                    asset_browser::asset_browser_ui,
                    layers_panel::layers_panel_ui,
                )
                    .chain(),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    // Second: top panels (after side panels)
                    toolbar::toolbar_ui,
                    toolbar::tool_settings_ui,
                )
                    .chain()
                    .after(asset_browser::asset_browser_ui)
                    .after(layers_panel::layers_panel_ui),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    // Last: dialogs/overlays
                    file_menu::file_menu_ui,
                    file_menu::missing_map_warning_ui,
                    file_menu::unsaved_changes_dialog_ui,
                    file_menu::async_operation_modal_ui,
                    file_menu::save_error_dialog_ui,
                    file_menu::save_validation_warning_ui,
                    file_menu::load_validation_warning_ui,
                    file_menu::config_reset_notification_ui,
                    asset_import::asset_import_ui,
                    layers_panel::help_popup_ui,
                    settings_dialog::settings_dialog_ui,
                )
                    .after(toolbar::toolbar_ui),
            )
            .add_systems(Update, file_menu::handle_window_close)
            .add_systems(
                EguiPrimaryContextPass,
                session_controls::monitor_selection_dialog,
            )
            .add_systems(Update, session_controls::enumerate_monitors)
            .add_systems(Update, layers_panel::handle_help_shortcut)
            // Update dialog state at the start of each frame
            .add_systems(First, update_dialog_state);
    }
}
