mod fog;
mod layer;
mod map_data;
pub mod persistence;
mod placed_item;

pub use fog::{FogOfWarData, SavedFogOfWar, cell_to_world, cells_in_radius, world_to_cell};
pub use layer::{Layer, render_layer};
pub use map_data::{
    MAP_FORMAT_VERSION, MapData, SavedAnnotations, SavedLine, SavedMap, SavedPath, SavedPlacedItem,
    SavedTextBox,
};
pub use persistence::{
    AsyncMapOperation, CurrentMapFile, LoadMapRequest, MapAssetReport, MapChangedOnDisk,
    MapDirtyState, MapLoadError, MapReplaced, MapSaveError, MapSaveFinished, NewMapRequest,
    OpenMaps, SaveConflict, SaveConflictDialog, SaveConflictKind, SaveMapRequest, SwitchMapRequest,
};
pub use placed_item::{ItemId, MissingAsset, PlacedItem, Selected, StackTops, Stacker};

use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapData>()
            .init_resource::<FogOfWarData>()
            .init_resource::<MapLoadError>()
            .init_resource::<MapSaveError>()
            .init_resource::<MapAssetReport>()
            .init_resource::<SaveConflictDialog>()
            .init_resource::<MapChangedOnDisk>()
            .init_resource::<CurrentMapFile>()
            .init_resource::<MapDirtyState>()
            .init_resource::<OpenMaps>()
            .init_resource::<AsyncMapOperation>()
            .add_message::<SaveMapRequest>()
            .add_message::<persistence::MapSaveFinished>()
            .add_message::<persistence::MapReplaced>()
            .add_message::<LoadMapRequest>()
            .add_message::<NewMapRequest>()
            .add_message::<SwitchMapRequest>()
            .add_systems(
                Update,
                (
                    persistence::save_map_system.run_if(on_message::<SaveMapRequest>),
                    persistence::load_map_system.run_if(on_message::<LoadMapRequest>),
                    persistence::new_map_system.run_if(on_message::<NewMapRequest>),
                    persistence::switch_map_system.run_if(on_message::<SwitchMapRequest>),
                    persistence::poll_save_tasks,
                    persistence::detect_open_map_changes
                        .run_if(on_message::<crate::assets::MapFilesChanged>),
                    persistence::poll_load_tasks,
                    // Change detection using Bevy's Added/Changed/Removed filters.
                    // Chained so suppression decays only after all three have
                    // observed the current frame's suppression value.
                    (
                        persistence::detect_item_additions,
                        persistence::detect_item_removals,
                        persistence::detect_item_transforms,
                        persistence::decay_dirty_suppression,
                    )
                        .chain(),
                ),
            );
    }
}
