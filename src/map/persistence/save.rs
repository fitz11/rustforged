//! Map save system and task polling.

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use futures_lite::future;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::assets::{AssetFingerprint, AssetLibrary, LibraryIndex};
use crate::config::UpdateLastMapPathRequest;
use crate::editor::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use crate::map::{FogOfWarData, MapData, PlacedItem, SavedMap};

use super::map_state::capture_current_map_state;
use super::messages::SaveMapRequest;
use super::resources::{
    AsyncMapOperation, CurrentMapFile, MapDirtyState, MapSaveError, OpenMaps, SaveMapTask,
};
use super::results::SaveResult;

/// Starts an async save operation
#[allow(clippy::too_many_arguments)]
pub fn save_map_system(
    mut commands: Commands,
    mut events: MessageReader<SaveMapRequest>,
    map_data: Res<MapData>,
    fog_data: Res<FogOfWarData>,
    placed_items: Query<(&PlacedItem, &Transform)>,
    paths: Query<&DrawnPath>,
    lines: Query<&DrawnLine>,
    texts: Query<(&Transform, &TextAnnotation)>,
    mut async_op: ResMut<AsyncMapOperation>,
    asset_library: Res<AssetLibrary>,
    open_maps: Res<OpenMaps>,
) {
    for event in events.read() {
        // Don't start a new save if one is already in progress
        if async_op.is_busy() {
            warn!("Save operation already in progress");
            continue;
        }

        // Placed items already hold library-relative paths, which is what map files store
        let mut saved_map =
            capture_current_map_state(&map_data, &fog_data, &placed_items, &paths, &lines, &texts);
        let used_assets = saved_map.used_assets();
        let mut index = asset_library.index();
        let carried = open_maps
            .active_map()
            .map(|m| m.asset_fingerprints.clone())
            .unwrap_or_default();

        let path = event.path.clone();
        let map_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map")
            .to_string();

        // Mark as saving
        async_op.is_saving = true;
        async_op.operation_description = Some(format!("Saving {}...", map_name));

        // Spawn async task for fingerprinting and file I/O
        let task_pool = IoTaskPool::get();
        let task = task_pool.spawn(async move {
            let fingerprints = fingerprint_assets(used_assets, &mut index, &carried);
            saved_map.asset_fingerprints = fingerprints.clone();

            let error = write_map(&path, &saved_map).err();
            SaveResult {
                path,
                success: error.is_none(),
                error,
                asset_fingerprints: fingerprints,
                computed: index.into_computed(),
            }
        });

        commands.spawn(SaveMapTask(task));
    }
}

/// Fingerprints every referenced asset. One that's missing right now (deleted, or not synced
/// to this machine yet) keeps the fingerprint the map was loaded with, so the reference can
/// still be found once the file shows up.
pub(super) fn fingerprint_assets(
    used_assets: BTreeSet<String>,
    index: &mut LibraryIndex,
    carried: &BTreeMap<String, AssetFingerprint>,
) -> BTreeMap<String, AssetFingerprint> {
    used_assets
        .into_iter()
        .filter_map(|asset| {
            let fp = index
                .fingerprint(&asset)
                .or_else(|| carried.get(&asset).cloned())?;
            Some((asset, fp))
        })
        .collect()
}

fn write_map(path: &Path, saved_map: &SavedMap) -> Result<(), String> {
    // Serialize (could be slow for large maps)
    let json = serde_json::to_string_pretty(saved_map)
        .map_err(|e| format!("Failed to serialize map: {}", e))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create maps folder: {}", e))?;
    }
    std::fs::write(path, json).map_err(|e| format!("Failed to write file: {}", e))
}

/// Polls save tasks and handles completion
#[allow(clippy::too_many_arguments)]
pub fn poll_save_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut SaveMapTask)>,
    mut async_op: ResMut<AsyncMapOperation>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut config_events: MessageWriter<UpdateLastMapPathRequest>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
    mut save_error: ResMut<MapSaveError>,
    mut asset_library: ResMut<AssetLibrary>,
    placed_items: Query<Entity, With<PlacedItem>>,
    annotations: Query<Entity, With<AnnotationMarker>>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            // Clear async state
            async_op.is_saving = false;
            async_op.operation_description = None;

            if !result.computed.is_empty() {
                asset_library.fingerprints.merge(result.computed);
            }

            if result.success {
                info!("Map saved to {:?}", result.path);

                // Clear any previous save error
                save_error.message = None;

                // Update current map file and config
                current_map_file.path = Some(result.path.clone());
                config_events.write(UpdateLastMapPathRequest {
                    path: result.path.clone(),
                });

                // Clear dirty state
                dirty_state.is_dirty = false;
                dirty_state.last_known_item_count = placed_items.iter().count();
                dirty_state.last_known_annotation_count = annotations.iter().count();

                // Update open maps
                if let Some(active_map) = open_maps.active_map_mut() {
                    active_map.is_dirty = false;
                    active_map.asset_fingerprints = result.asset_fingerprints;
                    active_map.path = Some(result.path.clone());
                    active_map.name = result
                        .path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                }
            } else if let Some(error) = result.error {
                error!("{}", error);
                // Store error for display to user
                save_error.message = Some(error);
            }

            commands.entity(entity).despawn();
        }
    }
}
