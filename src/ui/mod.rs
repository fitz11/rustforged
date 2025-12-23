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
            .add_systems(
                EguiPrimaryContextPass,
                (
                    file_menu::file_menu_ui,
                    toolbar::toolbar_ui,
                    asset_browser::asset_browser_ui,
                    layers_panel::layers_panel_ui,
                    asset_import::asset_import_ui,
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                session_controls::monitor_selection_dialog,
            )
            .add_systems(Update, session_controls::enumerate_monitors);
    }
}
