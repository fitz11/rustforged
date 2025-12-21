mod assets;
mod editor;
mod map;
mod session;
mod ui;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rustforged VTT Editor".into(),
                resolution: (1600, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(editor::EditorPlugin)
        .add_plugins(assets::AssetLibraryPlugin)
        .add_plugins(map::MapPlugin)
        .add_plugins(session::LiveSessionPlugin)
        .add_plugins(ui::UiPlugin)
        .run();
}
