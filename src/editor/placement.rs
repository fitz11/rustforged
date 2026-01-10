use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::assets::SelectedAsset;
use crate::map::{MapData, PlacedItem};

use super::params::{is_cursor_over_ui, CameraParams};
use super::tools::{CurrentTool, EditorTool, SelectedLayer};
use super::GridSettings;

#[allow(clippy::too_many_arguments)]
pub fn handle_placement(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    selected_layer: Res<SelectedLayer>,
    selected_asset: Res<SelectedAsset>,
    grid_settings: Res<GridSettings>,
    map_data: Res<MapData>,
    asset_server: Res<AssetServer>,
    camera: CameraParams,
    mut contexts: EguiContexts,
) {
    if current_tool.tool != EditorTool::Place {
        return;
    }

    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    // Don't place if clicking on UI
    if is_cursor_over_ui(&mut contexts) {
        return;
    }

    let Some(ref asset) = selected_asset.asset else {
        return;
    };

    let Some(world_pos) = camera.cursor_world_pos() else {
        return;
    };

    // Snap to grid unless Shift is held
    let snap_enabled = grid_settings.snap_enabled && !keyboard.pressed(KeyCode::ShiftLeft);
    let final_pos = super::grid::snap_to_grid(world_pos, map_data.grid_size, snap_enabled);

    // Use the selected layer instead of deriving from asset category
    let layer = selected_layer.layer;
    let z = layer.z_base();

    let texture: Handle<Image> = asset_server.load(&asset.relative_path);

    commands.spawn((
        Sprite::from_image(texture),
        Transform::from_translation(final_pos.extend(z)),
        PlacedItem {
            asset_path: asset.relative_path.clone(),
            layer,
            z_index: 0,
        },
    ));
}
