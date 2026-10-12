//! Freezing the player view.
//!
//! While frozen, players keep seeing the map as it was at the moment of
//! freezing, so the GM can edit it out of sight. Freezing copies every
//! player-visible item onto [`FROZEN_LAYER`] and points the player camera at
//! that layer instead of the live one. The player's fog and grid are drawn
//! from a copy of the map and fog data taken at the same moment (see
//! [`PlayerMapView`]).
//!
//! Whether viewport moves still reach the players while frozen is a setting
//! (`AppConfigData::move_player_view_while_frozen`); see `sync_player_camera`.

use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::map::{FogOfWarData, MapData, PlacedItem};

use super::player_window::PlayerCamera;
use super::state::{LiveSessionState, PlayerViewport};

/// Render layer for the frozen copies of placed items. Only the player camera
/// looks at it, and only while frozen.
pub const FROZEN_LAYER: usize = 3;

/// Render layers the player camera sees: live items (0) or their frozen copies,
/// plus the player-only fog and grid (2)
pub fn player_camera_layers(frozen: bool) -> RenderLayers {
    let items_layer = if frozen { FROZEN_LAYER } else { 0 };
    RenderLayers::from_layers(&[items_layer, 2])
}

/// Marker for the copy of a placed item shown to players while frozen.
/// Has no `PlacedItem`, so it's never saved, selected, or edited.
#[derive(Component)]
pub struct FrozenItem;

/// Map state captured when the player view was frozen
pub struct FrozenMap {
    map_data: MapData,
    fog: FogOfWarData,
}

/// The frozen map state, if the player view is frozen
#[derive(Resource, Default)]
pub struct FrozenPlayerView(Option<FrozenMap>);

impl FrozenPlayerView {
    pub fn is_frozen(&self) -> bool {
        self.0.is_some()
    }
}

/// The map as the player window shows it: the frozen copy while frozen,
/// otherwise the live map
#[derive(SystemParam)]
pub struct PlayerMapView<'w> {
    frozen: Res<'w, FrozenPlayerView>,
    map_data: Res<'w, MapData>,
    fog: Res<'w, FogOfWarData>,
    pub viewport: Res<'w, PlayerViewport>,
}

impl PlayerMapView<'_> {
    pub fn map_data(&self) -> &MapData {
        self.frozen
            .0
            .as_ref()
            .map_or(&self.map_data, |f| &f.map_data)
    }

    pub fn fog(&self) -> &FogOfWarData {
        self.frozen.0.as_ref().map_or(&self.fog, |f| &f.fog)
    }
}

/// Take or release the frozen copy of the map when `LiveSessionState::frozen`
/// changes, and point the player camera at the matching layer
#[allow(clippy::too_many_arguments)]
pub fn sync_freeze(
    mut commands: Commands,
    mut session_state: ResMut<LiveSessionState>,
    mut frozen_view: ResMut<FrozenPlayerView>,
    map_data: Res<MapData>,
    fog: Res<FogOfWarData>,
    items: Query<(&Sprite, &Transform, &Visibility, Option<&RenderLayers>), With<PlacedItem>>,
    frozen_items: Query<Entity, With<FrozenItem>>,
    mut player_cameras: Query<&mut RenderLayers, (With<PlayerCamera>, Without<PlacedItem>)>,
) {
    // A session always starts unfrozen
    if !session_state.is_active && session_state.frozen {
        session_state.frozen = false;
    }

    if session_state.frozen && !frozen_view.is_frozen() {
        let live_layer = RenderLayers::layer(0);
        for (sprite, transform, visibility, layers) in &items {
            let player_visible = layers.is_none_or(|layers| layers.intersects(&live_layer));
            if *visibility == Visibility::Hidden || !player_visible {
                continue;
            }
            commands.spawn((
                sprite.clone(),
                *transform,
                RenderLayers::layer(FROZEN_LAYER),
                FrozenItem,
            ));
        }
        frozen_view.0 = Some(FrozenMap {
            map_data: map_data.clone(),
            fog: fog.clone(),
        });
        info!("Froze the player view");
    } else if !session_state.frozen && frozen_view.is_frozen() {
        for entity in &frozen_items {
            commands.entity(entity).despawn();
        }
        frozen_view.0 = None;
        info!("Unfroze the player view");
    }

    // Every frame, so a camera spawned mid-freeze gets the right layers too
    let wanted = player_camera_layers(frozen_view.is_frozen());
    for mut layers in &mut player_cameras {
        if *layers != wanted {
            *layers = wanted.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_camera_layers_live() {
        let layers = player_camera_layers(false);
        assert!(layers.intersects(&RenderLayers::layer(0)));
        assert!(layers.intersects(&RenderLayers::layer(2)));
        assert!(!layers.intersects(&RenderLayers::layer(FROZEN_LAYER)));
    }

    #[test]
    fn test_player_camera_layers_frozen() {
        let layers = player_camera_layers(true);
        assert!(!layers.intersects(&RenderLayers::layer(0)));
        assert!(layers.intersects(&RenderLayers::layer(2)));
        assert!(layers.intersects(&RenderLayers::layer(FROZEN_LAYER)));
    }

    #[test]
    fn test_editor_never_sees_frozen_items() {
        // The editor camera renders layers 0 and 1
        let editor = RenderLayers::from_layers(&[0, 1]);
        assert!(!editor.intersects(&RenderLayers::layer(FROZEN_LAYER)));
    }

    fn freeze_test_app() -> App {
        let mut app = App::new();
        app.init_resource::<LiveSessionState>()
            .init_resource::<FrozenPlayerView>()
            .init_resource::<MapData>()
            .init_resource::<FogOfWarData>()
            .add_systems(Update, sync_freeze);
        app.world_mut().resource_mut::<LiveSessionState>().is_active = true;
        app
    }

    fn spawn_item(app: &mut App, layer: crate::map::Layer, render_layer: usize) -> Entity {
        app.world_mut()
            .spawn((
                Sprite::default(),
                Transform::from_xyz(35.0, 35.0, layer.z_base()),
                PlacedItem {
                    asset_path: "token.png".to_string(),
                    layer,
                    z_index: 0,
                },
                RenderLayers::layer(render_layer),
            ))
            .id()
    }

    fn frozen_item_count(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<FrozenItem>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn test_freeze_copies_player_visible_items_only() {
        use crate::map::Layer;

        let mut app = freeze_test_app();
        spawn_item(&mut app, Layer::Token, 0);
        spawn_item(&mut app, Layer::GM, 1);
        let hidden = spawn_item(&mut app, Layer::Terrain, 0);
        app.world_mut()
            .entity_mut(hidden)
            .insert(Visibility::Hidden);

        app.world_mut().resource_mut::<LiveSessionState>().frozen = true;
        app.update();

        assert!(app.world().resource::<FrozenPlayerView>().is_frozen());
        assert_eq!(frozen_item_count(&mut app), 1);
    }

    #[test]
    fn test_frozen_map_ignores_later_edits() {
        let mut app = freeze_test_app();
        app.world_mut()
            .resource_mut::<FogOfWarData>()
            .revealed_cells
            .insert((1, 1));

        app.world_mut().resource_mut::<LiveSessionState>().frozen = true;
        app.update();

        // Edit the live map after freezing
        app.world_mut()
            .resource_mut::<FogOfWarData>()
            .revealed_cells
            .insert((2, 2));
        app.world_mut().resource_mut::<MapData>().grid_visible = false;

        let frozen_view = app.world().resource::<FrozenPlayerView>();
        let frozen = frozen_view.0.as_ref().unwrap();
        assert!(frozen.fog.is_cell_revealed((1, 1)));
        assert!(!frozen.fog.is_cell_revealed((2, 2)));
        assert!(frozen.map_data.grid_visible);
    }

    #[test]
    fn test_unfreeze_removes_frozen_items() {
        let mut app = freeze_test_app();
        spawn_item(&mut app, crate::map::Layer::Token, 0);

        app.world_mut().resource_mut::<LiveSessionState>().frozen = true;
        app.update();
        assert_eq!(frozen_item_count(&mut app), 1);

        app.world_mut().resource_mut::<LiveSessionState>().frozen = false;
        app.update();
        assert_eq!(frozen_item_count(&mut app), 0);
        assert!(!app.world().resource::<FrozenPlayerView>().is_frozen());
    }

    #[test]
    fn test_ending_session_unfreezes() {
        let mut app = freeze_test_app();
        spawn_item(&mut app, crate::map::Layer::Token, 0);
        app.world_mut().resource_mut::<LiveSessionState>().frozen = true;
        app.update();

        app.world_mut().resource_mut::<LiveSessionState>().is_active = false;
        app.update();

        assert!(!app.world().resource::<LiveSessionState>().frozen);
        assert!(!app.world().resource::<FrozenPlayerView>().is_frozen());
        assert_eq!(frozen_item_count(&mut app), 0);
    }

    #[test]
    fn test_player_camera_follows_freeze() {
        let mut app = freeze_test_app();
        let camera = app
            .world_mut()
            .spawn((PlayerCamera, player_camera_layers(false)))
            .id();

        app.world_mut().resource_mut::<LiveSessionState>().frozen = true;
        app.update();
        assert_eq!(
            app.world().get::<RenderLayers>(camera),
            Some(&player_camera_layers(true))
        );

        app.world_mut().resource_mut::<LiveSessionState>().frozen = false;
        app.update();
        assert_eq!(
            app.world().get::<RenderLayers>(camera),
            Some(&player_camera_layers(false))
        );
    }
}
