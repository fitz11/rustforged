//! Place tool for single-click asset placement.
//!
//! ## Tool Behavior
//!
//! - A translucent ghost of the selected asset follows the cursor, showing where
//!   it will land (the Brush tool shows the same ghost)
//! - Click: Places the asset, then switches to the Select tool with it selected
//! - Shift+click: Places the asset and stays in the Place tool for another one
//! - Ctrl+click: Free placement - puts the asset under the cursor instead of
//!   snapping it to the grid cell center

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::assets::SelectedAsset;
use crate::input::{command_held, shift_held};
use crate::map::{MapData, PlacedItem, Selected, StackTops, render_layer};
use crate::ui::DialogState;

use super::GridSettings;
use super::brush::{BrushState, fit_to_cell_scale};
use super::grid::snap_to_grid;
use super::history::{EditorCommand, MapObject, RecordEditorCommand};
use super::params::{CameraParams, is_cursor_over_ui};
use super::tools::{CurrentTool, EditorTool, SelectedLayer};

/// Opacity of the placement ghost
const GHOST_ALPHA: f32 = 0.5;
/// Above every layer (Play tops out at z=424) so the ghost is never hidden
const GHOST_Z: f32 = 500.0;

/// Translucent preview of the selected asset that follows the cursor while the
/// Place or Brush tool is active. Has no `PlacedItem`, so it's never saved,
/// selected, or hit-tested.
#[derive(Component, Default)]
pub struct PlacementGhost {
    /// Library path of the asset the ghost's sprite currently shows
    asset_path: Option<String>,
}

pub fn spawn_placement_ghost(mut commands: Commands) {
    commands.spawn((
        PlacementGhost::default(),
        Sprite {
            color: Color::srgba(1.0, 1.0, 1.0, GHOST_ALPHA),
            ..default()
        },
        Transform::default(),
        Visibility::Hidden,
        // Editor-only, so players never see it
        RenderLayers::layer(render_layer::EDITOR),
    ));
}

/// Where a Place click at `world_pos` puts the asset: the grid cell's center,
/// or exactly under the cursor while Ctrl is held
fn placement_position(
    world_pos: Vec2,
    keyboard: &ButtonInput<KeyCode>,
    grid_settings: &GridSettings,
    grid_size: f32,
) -> Vec2 {
    let ctrl_held = command_held(keyboard);
    snap_to_grid(
        world_pos,
        grid_size,
        grid_settings.snap_enabled && !ctrl_held,
    )
}

/// Move the ghost to where the active tool would place the selected asset,
/// hiding it whenever a click wouldn't place anything
#[allow(clippy::too_many_arguments)]
pub fn update_placement_ghost(
    keyboard: Res<ButtonInput<KeyCode>>,
    current_tool: Res<CurrentTool>,
    selected_asset: Res<SelectedAsset>,
    grid_settings: Res<GridSettings>,
    map_data: Res<MapData>,
    dialog_state: Res<DialogState>,
    brush_state: Res<BrushState>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    camera: CameraParams,
    mut contexts: EguiContexts,
    mut ghost_query: Query<(
        &mut PlacementGhost,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let Ok((mut ghost, mut sprite, mut transform, mut visibility)) = ghost_query.single_mut()
    else {
        return;
    };

    let placing = match current_tool.tool {
        EditorTool::Place => true,
        // Mid-stroke the cursor sits on items just placed, so the ghost only
        // adds noise
        EditorTool::Brush => !brush_state.is_brushing,
        _ => false,
    };
    let target = if placing && !dialog_state.any_modal_open && !is_cursor_over_ui(&mut contexts) {
        selected_asset.asset.as_ref().zip(camera.cursor_world_pos())
    } else {
        None
    };
    let Some((asset, world_pos)) = target else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };

    if ghost.asset_path.as_deref() != Some(asset.path.as_str()) {
        sprite.image = asset_server.load(asset.asset_path());
        ghost.asset_path = Some(asset.path.clone());
    }

    let grid_size = map_data.grid_size;
    let (position, scale) = if current_tool.tool == EditorTool::Brush {
        // Mirror handle_brush: always snapped, Shift fits the asset to one cell
        let shift_held = shift_held(&keyboard);
        let scale = match images.get(&sprite.image) {
            Some(image) if shift_held => fit_to_cell_scale(image.size_f32(), grid_size),
            _ => Vec3::ONE,
        };
        (snap_to_grid(world_pos, grid_size, true), scale)
    } else {
        let position = placement_position(world_pos, &keyboard, &grid_settings, grid_size);
        (position, Vec3::ONE)
    };

    transform.set_if_neq(Transform {
        translation: position.extend(GHOST_Z),
        scale,
        ..default()
    });
    visibility.set_if_neq(Visibility::Inherited);
}

#[allow(clippy::too_many_arguments)]
pub fn handle_placement(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut current_tool: ResMut<CurrentTool>,
    selected_layer: Res<SelectedLayer>,
    selected_asset: Res<SelectedAsset>,
    grid_settings: Res<GridSettings>,
    map_data: Res<MapData>,
    asset_server: Res<AssetServer>,
    camera: CameraParams,
    mut contexts: EguiContexts,
    mut history_writer: MessageWriter<RecordEditorCommand>,
    selected_query: Query<Entity, With<Selected>>,
    stack: StackTops,
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

    // A locked or hidden layer takes no new items (the tool settings bar says so)
    if !map_data.is_layer_editable(selected_layer.layer) {
        return;
    }

    let Some(world_pos) = camera.cursor_world_pos() else {
        return;
    };

    let final_pos = placement_position(world_pos, &keyboard, &grid_settings, map_data.grid_size);

    // Use the selected layer instead of deriving from asset category
    let item = PlacedItem {
        asset_path: asset.path.clone(),
        layer: selected_layer.layer,
        z_index: 0,
    };
    // On top of what's already on the layer
    let z = stack.z_on_top(&item);
    let object = MapObject::new_item(item, Transform::from_translation(final_pos.extend(z)));
    let entity = object.spawn(&mut commands, &asset_server);
    history_writer.write(RecordEditorCommand {
        command: EditorCommand::Add(vec![object]),
    });

    // Shift keeps the Place tool for another placement; otherwise hand off to
    // Select with the new item selected so it can be adjusted right away
    let shift_held = shift_held(&keyboard);
    if !shift_held {
        for selected in selected_query.iter() {
            commands.entity(selected).remove::<Selected>();
        }
        commands.entity(entity).insert(Selected);
        current_tool.tool = EditorTool::Select;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRID: f32 = 70.0;

    fn keyboard_with(keys: &[KeyCode]) -> ButtonInput<KeyCode> {
        let mut keyboard = ButtonInput::default();
        for key in keys {
            keyboard.press(*key);
        }
        keyboard
    }

    #[test]
    fn test_placement_snaps_to_cell_center_by_default() {
        let pos = placement_position(
            Vec2::new(10.0, 20.0),
            &keyboard_with(&[]),
            &GridSettings::default(),
            GRID,
        );
        assert_eq!(pos, Vec2::new(35.0, 35.0));
    }

    #[test]
    fn test_placement_ctrl_places_freely() {
        for ctrl in [KeyCode::ControlLeft, KeyCode::ControlRight] {
            let pos = placement_position(
                Vec2::new(10.0, 20.0),
                &keyboard_with(&[ctrl]),
                &GridSettings::default(),
                GRID,
            );
            assert_eq!(pos, Vec2::new(10.0, 20.0));
        }
    }

    #[test]
    fn test_placement_shift_still_snaps() {
        let pos = placement_position(
            Vec2::new(10.0, 20.0),
            &keyboard_with(&[KeyCode::ShiftLeft]),
            &GridSettings::default(),
            GRID,
        );
        assert_eq!(pos, Vec2::new(35.0, 35.0));
    }

    #[test]
    fn test_placement_respects_snap_disabled() {
        let pos = placement_position(
            Vec2::new(10.0, 20.0),
            &keyboard_with(&[]),
            &GridSettings {
                snap_enabled: false,
            },
            GRID,
        );
        assert_eq!(pos, Vec2::new(10.0, 20.0));
    }
}
