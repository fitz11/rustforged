mod asset_browser;
pub mod asset_import;
pub mod file_menu;
mod layers_panel;
mod session_controls;
mod toolbar;

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<asset_browser::AssetBrowserState>()
            .init_resource::<asset_import::AssetImportDialog>()
            .init_resource::<file_menu::FileMenuState>()
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
                    asset_import::asset_import_ui,
                )
                    .after(toolbar::toolbar_ui),
            )
            .add_systems(
                EguiPrimaryContextPass,
                session_controls::monitor_selection_dialog,
            )
            .add_systems(Update, session_controls::enumerate_monitors);
    }
}
