mod library;
mod placeholder;
mod validation;
pub use library::{
    create_and_open_library, get_image_dimensions, load_thumbnail, open_library_directory,
    save_library_metadata, AssetLibrary, LibraryAsset, ThumbnailCache, THUMBNAIL_SIZE,
};

use bevy::prelude::*;

use crate::config::{AppConfig, ConfigLoaded};
use crate::map::PlacedItem;

#[derive(Message)]
pub struct RefreshAssetLibrary;

/// Message to update library metadata (name)
#[derive(Message)]
pub struct UpdateLibraryMetadataRequest {
    pub name: String,
}

/// Message to update placed items after an asset is renamed
#[derive(Message)]
pub struct RenameAssetRequest {
    pub old_path: String,
    pub new_path: String,
}

pub struct AssetLibraryPlugin;

impl Plugin for AssetLibraryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetLibrary>()
            .init_resource::<SelectedAsset>()
            .init_resource::<ThumbnailCache>()
            .add_message::<RefreshAssetLibrary>()
            .add_message::<UpdateLibraryMetadataRequest>()
            .add_message::<RenameAssetRequest>()
            .add_systems(
                Startup,
                (
                    placeholder::setup_placeholder_texture,
                    init_library_from_config,
                    library::scan_asset_library,
                )
                    .chain()
                    .after(ConfigLoaded),
            )
            .add_systems(
                Update,
                (
                    library::refresh_asset_library.run_if(on_message::<RefreshAssetLibrary>),
                    library::track_library_changes.run_if(resource_changed::<AssetLibrary>),
                    update_library_metadata_system
                        .run_if(on_message::<UpdateLibraryMetadataRequest>),
                    rename_asset_system.run_if(on_message::<RenameAssetRequest>),
                    // Check for missing assets periodically (runs on entities without MissingAsset marker)
                    validation::detect_missing_assets,
                    // Draw indicators for missing assets
                    validation::draw_missing_asset_indicators,
                ),
            );
    }
}

/// System to update library metadata when requested
fn update_library_metadata_system(
    mut events: MessageReader<UpdateLibraryMetadataRequest>,
    mut library: ResMut<AssetLibrary>,
) {
    for event in events.read() {
        library.metadata.name = event.name.clone();
        if let Err(e) = save_library_metadata(&library.library_path, &library.metadata) {
            warn!("Failed to save library metadata: {}", e);
        }
    }
}

/// System to update placed items when an asset is renamed
fn rename_asset_system(
    mut events: MessageReader<RenameAssetRequest>,
    mut placed_items: Query<&mut PlacedItem>,
) {
    for event in events.read() {
        let mut count = 0;
        for mut item in placed_items.iter_mut() {
            if item.asset_path == event.old_path {
                item.asset_path = event.new_path.clone();
                count += 1;
            }
        }
        if count > 0 {
            info!(
                "Updated {} placed item(s): {} -> {}",
                count, event.old_path, event.new_path
            );
        }
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
