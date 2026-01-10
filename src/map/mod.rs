mod layer;
mod map_data;
pub mod persistence;
mod placed_item;

pub use layer::Layer;
pub use map_data::{
    MapData, SavedAnnotations, SavedLine, SavedMap, SavedPath, SavedPlacedItem, SavedTextBox,
};
pub use persistence::{LoadMapRequest, MapLoadError, NewMapRequest, SaveMapRequest};
pub use placed_item::{PlacedItem, Selected};

use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapData>()
            .init_resource::<MapLoadError>()
            .add_message::<SaveMapRequest>()
            .add_message::<LoadMapRequest>()
            .add_message::<NewMapRequest>()
            .add_systems(Startup, persistence::ensure_maps_directory)
            .add_systems(
                Update,
                (
                    persistence::save_map_system.run_if(on_message::<SaveMapRequest>),
                    persistence::load_map_system.run_if(on_message::<LoadMapRequest>),
                    persistence::new_map_system.run_if(on_message::<NewMapRequest>),
                ),
            );
    }
}
