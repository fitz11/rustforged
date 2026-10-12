mod asset_browser;
pub mod asset_import;
pub mod file_menu;
mod layers_panel;
pub mod map_actions;
pub mod panel_layout;
mod session_controls;
mod settings_dialog;
pub mod shortcuts;
mod toolbar;

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::map::AsyncMapOperation;

/// Resource that tracks whether any modal dialog is currently open.
/// Editor input handlers should check this to avoid processing input
/// when the user is interacting with a dialog.
#[derive(Resource, Default)]
pub struct DialogState {
    /// True when any modal dialog is open that should block editor input
    pub any_modal_open: bool,
    /// True when a text field or number field has keyboard focus, so typing must not
    /// trigger shortcuts
    pub text_input_focused: bool,
}

/// Docked panels of the main editor layout. Each panel takes space from what the panels
/// before it left over, so systems adding more panels should run after this set.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditorPanels;

/// Works out, at the start of each frame, whether a dialog is open and whether a text field
/// has the keyboard, for the editor's input systems.
///
/// Every dialog is an egui window, so "a dialog is open" is "egui showed a window last
/// frame": a new dialog is covered without being listed here. Native file pickers aren't
/// egui windows, and a save or load in progress blocks input too.
fn update_dialog_state(
    mut contexts: bevy_egui::EguiContexts,
    asset_browser: Res<asset_browser::AssetBrowserState>,
    asset_import: Res<asset_import::AssetImportDialog>,
    settings: Res<settings_dialog::SettingsDialogState>,
    open_map_picker: Res<map_actions::OpenMapPicker>,
    async_op: Res<AsyncMapOperation>,
    mut dialog_state: ResMut<DialogState>,
) {
    let ctx = contexts.ctx_mut().ok();
    let egui_window_open = ctx.as_ref().is_some_and(|ctx| {
        ctx.memory(|memory| {
            memory
                .areas()
                .visible_layer_ids()
                .iter()
                .any(|layer| layer.order == bevy_egui::egui::Order::Middle)
        })
    });
    let native_picker_open = asset_browser.any_file_dialog_pending()
        || asset_import.pending_browse.is_some()
        || settings.pending_browse.is_some()
        || open_map_picker.is_open();

    dialog_state.any_modal_open = egui_window_open || native_picker_open || async_op.is_busy();
    dialog_state.text_input_focused = ctx.is_some_and(|ctx| ctx.egui_wants_keyboard_input());
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogState>()
            .init_resource::<asset_browser::AssetBrowserState>()
            .init_resource::<asset_import::AssetImportDialog>()
            .init_resource::<file_menu::FileMenuState>()
            .init_resource::<map_actions::UnsavedChangesPrompt>()
            .init_resource::<map_actions::OpenMapPicker>()
            .add_message::<map_actions::MapFileAction>()
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
                    .chain()
                    .in_set(EditorPanels),
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
                    .after(layers_panel::layers_panel_ui)
                    .in_set(EditorPanels),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    // Last: dialogs/overlays
                    file_menu::file_menu_ui,
                    file_menu::missing_map_warning_ui,
                    map_actions::unsaved_changes_dialog_ui,
                    file_menu::async_operation_modal_ui,
                    file_menu::save_error_dialog_ui,
                    file_menu::map_asset_report_ui,
                    file_menu::save_conflict_dialog_ui,
                    file_menu::config_reset_notification_ui,
                    asset_import::asset_import_ui,
                    layers_panel::help_popup_ui,
                    settings_dialog::settings_dialog_ui,
                )
                    .after(toolbar::toolbar_ui),
            )
            .add_systems(
                Update,
                (
                    map_actions::handle_file_shortcuts.run_if(crate::editor::shortcuts_enabled),
                    map_actions::poll_open_map_picker,
                    map_actions::handle_map_file_actions,
                    map_actions::run_action_after_save,
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                session_controls::monitor_selection_dialog,
            )
            .add_systems(Update, session_controls::enumerate_monitors)
            .add_systems(
                Update,
                layers_panel::handle_help_shortcut.run_if(
                    crate::editor::plain_key(KeyCode::KeyH).and_then(crate::editor::no_text_input),
                ),
            )
            // Update dialog state at the start of each frame
            .add_systems(First, update_dialog_state);
    }
}
