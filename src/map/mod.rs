mod fog;
mod layer;
mod map_data;
pub mod persistence;
mod placed_item;

pub use fog::{cell_to_world, cells_in_radius, world_to_cell, FogOfWarData, SavedFogOfWar};
pub use layer::Layer;
pub use map_data::{
    AssetManifest, MapData, SavedAnnotations, SavedLine, SavedMap, SavedPath, SavedPlacedItem,
    SavedTextBox,
};
pub use persistence::{
    AsyncMapOperation, CurrentMapFile, LoadMapRequest, LoadValidationWarning, MapDirtyState,
    MapLoadError, MapSaveError, NewMapRequest, OpenMaps, SaveMapRequest, SaveValidationWarning,
    SwitchMapRequest, UnsavedChangesDialog,
};
pub use placed_item::{MissingAsset, PlacedItem, Selected};

use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapData>()
            .init_resource::<FogOfWarData>()
            .init_resource::<MapLoadError>()
            .init_resource::<MapSaveError>()
            .init_resource::<SaveValidationWarning>()
            .init_resource::<LoadValidationWarning>()
            .init_resource::<CurrentMapFile>()
            .init_resource::<MapDirtyState>()
            .init_resource::<OpenMaps>()
            .init_resource::<UnsavedChangesDialog>()
            .init_resource::<AsyncMapOperation>()
            .add_message::<SaveMapRequest>()
            .add_message::<LoadMapRequest>()
            .add_message::<NewMapRequest>()
            .add_message::<SwitchMapRequest>()
            .add_systems(Startup, persistence::ensure_maps_directory)
            .add_systems(
                Update,
                (
                    persistence::save_map_system.run_if(on_message::<SaveMapRequest>),
                    persistence::load_map_system.run_if(on_message::<LoadMapRequest>),
                    persistence::new_map_system.run_if(on_message::<NewMapRequest>),
                    persistence::switch_map_system.run_if(on_message::<SwitchMapRequest>),
                    persistence::poll_save_tasks,
                    persistence::poll_load_tasks,
                    // Change detection using Bevy's Added/Changed/Removed filters
                    persistence::detect_item_additions,
                    persistence::detect_item_removals,
                    persistence::detect_item_transforms,
                ),
            );
    }
}
