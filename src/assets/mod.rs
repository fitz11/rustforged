mod asset_type;
mod library;

pub use asset_type::AssetCategory;
pub use library::{
    create_and_open_library, get_image_dimensions, open_library_directory, AssetLibrary,
    LibraryAsset,
};

use bevy::prelude::*;

#[derive(Message)]
pub struct RefreshAssetLibrary;

pub struct AssetLibraryPlugin;

impl Plugin for AssetLibraryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetLibrary>()
            .init_resource::<SelectedAsset>()
            .add_message::<RefreshAssetLibrary>()
            .add_systems(Startup, library::scan_asset_library)
            .add_systems(
                Update,
                library::refresh_asset_library.run_if(on_message::<RefreshAssetLibrary>),
            );
    }
}

#[derive(Resource, Default)]
pub struct SelectedAsset {
    pub asset: Option<LibraryAsset>,
}
