mod asset_type;
mod library;

pub use asset_type::AssetCategory;
pub use library::{
    create_and_open_library, get_image_dimensions, load_thumbnail, open_library_directory,
    AssetLibrary, LibraryAsset, ThumbnailCache, THUMBNAIL_SIZE,
};

use bevy::prelude::*;

use crate::config::{AppConfig, ConfigLoaded};

#[derive(Message)]
pub struct RefreshAssetLibrary;

pub struct AssetLibraryPlugin;

impl Plugin for AssetLibraryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetLibrary>()
            .init_resource::<SelectedAsset>()
            .init_resource::<ThumbnailCache>()
            .add_message::<RefreshAssetLibrary>()
            .add_systems(
                Startup,
                (init_library_from_config, library::scan_asset_library)
                    .chain()
                    .after(ConfigLoaded),
            )
            .add_systems(
                Update,
                (
                    library::refresh_asset_library.run_if(on_message::<RefreshAssetLibrary>),
                    library::track_library_changes.run_if(resource_changed::<AssetLibrary>),
                ),
            );
    }
}

/// Initialize asset library from config (runs before scan_asset_library)
fn init_library_from_config(config: Res<AppConfig>, mut library: ResMut<AssetLibrary>) {
    // Use default library path from config if available and valid
    if let Some(ref path) = config.data.default_library_path {
        if path.exists() {
            match open_library_directory(&mut library, path.clone()) {
                Ok(()) => info!("Opened default library from config: {:?}", path),
                Err(e) => warn!("Failed to open saved default library: {}", e),
            }
        } else {
            warn!("Default library path no longer exists: {:?}", path);
        }
    }
    // Otherwise, default library path from AssetLibrary::default() is used
}

#[derive(Resource, Default)]
pub struct SelectedAsset {
    pub asset: Option<LibraryAsset>,
}
