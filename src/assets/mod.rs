mod cache;
mod fingerprint;
mod library;
mod manifest;
mod placeholder;
mod source;
mod validation;
mod watcher;
pub use cache::{LibraryCache, load_thumbnail};
#[cfg(test)]
pub use fingerprint::test_support as fingerprint_test_support;
pub use fingerprint::{AssetFingerprint, FingerprintCache, LibraryIndex};
pub use library::relative_to_library;
pub use library::{
    AssetLibrary, LibraryAsset, THUMBNAIL_SIZE, ThumbnailCache, create_and_open_library,
    get_image_dimensions, open_library_directory,
};
pub use manifest::{LIBRARY_MANIFEST_FILE, is_root_manifest, save_library_metadata};
pub use source::{LibraryRoot, library_asset_path, register_library_source};
pub use watcher::MapFilesChanged;

use bevy::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::{AppConfig, ConfigLoaded};
use crate::editor::Clipboard;
use crate::editor::history::CommandHistory;
use crate::map::{MapDirtyState, MissingAsset, OpenMaps, PlacedItem};

#[derive(Message)]
pub struct RefreshAssetLibrary;

/// Message to update library metadata (name)
#[derive(Message)]
pub struct UpdateLibraryMetadataRequest {
    pub name: String,
}

/// Message to point everything that references one library-relative asset path at another:
/// placed items, background open maps, the clipboard and undo history. Sent after an asset
/// is renamed or moved in the app, or when the user relinks a missing asset.
#[derive(Message)]
pub struct RetargetAssetRequest {
    pub old_path: String,
    pub new_path: String,
}

pub struct AssetLibraryPlugin;

impl Plugin for AssetLibraryPlugin {
    fn build(&self, app: &mut App) {
        // Share the `lib://` source's root (registered before DefaultPlugins in main) so
        // opening a library also redirects asset loads.
        let root = app
            .world()
            .get_resource::<LibraryRoot>()
            .cloned()
            .unwrap_or_default();
        app.insert_resource(AssetLibrary::with_root(root))
            .init_resource::<SelectedAsset>()
            .init_resource::<ThumbnailCache>()
            .add_message::<RefreshAssetLibrary>()
            .add_message::<UpdateLibraryMetadataRequest>()
            .add_message::<RetargetAssetRequest>()
            .add_message::<MapFilesChanged>()
            .init_resource::<watcher::LibraryWatcher>()
            .init_resource::<LibraryCache>()
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
                    retarget_asset_system.run_if(on_message::<RetargetAssetRequest>),
                    reload_placed_items_on_library_switch,
                    (
                        watcher::watch_library_root,
                        watcher::process_library_changes,
                    )
                        .chain(),
                    (cache::open_library_cache, cache::save_fingerprint_cache).chain(),
                    // Check for missing assets periodically (runs on entities without MissingAsset marker)
                    validation::detect_missing_assets,
                    // Bring placeholders back once their file shows up in the library
                    validation::restore_found_assets,
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
        let library = &mut *library;
        library.metadata.name = event.name.clone();
        if let Err(e) = save_library_metadata(&library.library_path, &mut library.metadata) {
            warn!("Failed to save library metadata: {}", e);
        }
    }
}

/// System to point every in-memory reference to an asset at its new path
#[allow(clippy::too_many_arguments)]
fn retarget_asset_system(
    mut commands: Commands,
    mut events: MessageReader<RetargetAssetRequest>,
    asset_server: Res<AssetServer>,
    mut placed_items: Query<(Entity, &mut PlacedItem, &mut Sprite, Has<MissingAsset>)>,
    mut open_maps: ResMut<OpenMaps>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut clipboard: ResMut<Clipboard>,
    mut history: ResMut<CommandHistory>,
) {
    for event in events.read() {
        let mut count = 0;
        for (entity, mut item, mut sprite, missing) in placed_items.iter_mut() {
            if item.asset_path != event.old_path {
                continue;
            }
            item.asset_path = event.new_path.clone();
            if missing {
                // Drop the placeholder; detect_missing_assets re-checks the new path.
                commands.entity(entity).remove::<MissingAsset>();
                *sprite =
                    Sprite::from_image(asset_server.load(library_asset_path(&item.asset_path)));
            } else {
                sprite.image = asset_server.load(library_asset_path(&item.asset_path));
            }
            count += 1;
        }

        // The map files still name the old path until saved (they'd find the asset again
        // by fingerprint anyway), so changed maps count as unsaved.
        if count > 0 {
            dirty_state.mark_dirty();
        }

        // Maps open in the background keep their items in a snapshot, not as entities
        let active_id = open_maps.active_map_id;
        for map in open_maps.maps.values_mut() {
            if Some(map.id) != active_id
                && let Some(state) = map.saved_state.as_mut()
                && state.retarget_asset(&event.old_path, &event.new_path)
            {
                map.stashed_dirty = true;
            }
        }
        clipboard.retarget_asset(&event.old_path, &event.new_path);
        history.retarget_asset(&event.old_path, &event.new_path);

        if count > 0 {
            info!(
                "Updated {} placed item(s): {} -> {}",
                count, event.old_path, event.new_path
            );
        }
    }
}

/// `lib://` paths resolve against the open library, so when the user switches libraries
/// with a map open, reload its images from the new library. Items whose asset isn't there
/// fail to load and get placeholders.
fn reload_placed_items_on_library_switch(
    library: Res<AssetLibrary>,
    mut last_path: Local<Option<PathBuf>>,
    asset_server: Res<AssetServer>,
    placed_items: Query<&PlacedItem>,
) {
    if last_path.as_ref() == Some(&library.library_path) {
        return;
    }
    let switched = last_path.is_some();
    *last_path = Some(library.library_path.clone());
    if !switched {
        return;
    }

    let paths: HashSet<&str> = placed_items.iter().map(|i| i.asset_path.as_str()).collect();
    for path in paths {
        asset_server.reload(library_asset_path(path));
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
