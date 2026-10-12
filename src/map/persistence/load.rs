//! Map load system and task polling.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use std::path::PathBuf;

use crate::assets::{AssetLibrary, LibraryIndex};
use crate::config::UpdateLastMapPathRequest;
use crate::editor::AnnotationMarker;
use crate::map::{FogOfWarData, MAP_FORMAT_VERSION, MapData, PlacedItem, SavedMap};

use super::map_state::spawn_map_contents;
use super::messages::{LoadMapRequest, MapReplaced};
use super::resources::{
    AsyncMapOperation, CurrentMapFile, LoadMapTask, MapAssetReport, MapChangedOnDisk,
    MapDirtyState, MapLoadError, OpenMap, OpenMaps,
};
use super::results::LoadResult;
use crate::common::poll_task;

/// Starts an async load operation (file I/O and asset reference resolution)
pub fn load_map_system(
    mut commands: Commands,
    mut events: MessageReader<LoadMapRequest>,
    mut async_op: ResMut<AsyncMapOperation>,
    asset_library: Res<AssetLibrary>,
    mut waiting: Local<Option<LoadMapRequest>>,
) {
    // A request made while another save or load runs waits for it (the latest one wins)
    for event in events.read() {
        *waiting = Some(event.clone());
    }
    if async_op.is_busy() {
        return;
    }
    if let Some(event) = waiting.take() {
        let path = event.path.clone();
        let map_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map")
            .to_string();

        // Mark as loading
        async_op.is_loading = true;
        async_op.operation_description = Some(format!("Loading {}...", map_name));

        let index = asset_library.index();

        // Spawn async task for file I/O, parsing and resolving asset references
        let task_pool = IoTaskPool::get();
        let task = task_pool.spawn(async move { read_map(path, index) });

        commands.spawn(LoadMapTask(task));
    }
}

pub(super) fn read_map(path: PathBuf, mut index: LibraryIndex) -> LoadResult {
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e));
    let disk_hash = json
        .as_ref()
        .ok()
        .map(|json| crate::fsutil::content_hash(json.as_bytes()));
    let parsed = json
        .and_then(|json| {
            serde_json::from_str::<SavedMap>(&json)
                .map_err(|e| format!("Failed to parse map file: {}", e))
        })
        .and_then(|map| {
            if map.format_version > MAP_FORMAT_VERSION {
                Err(format!(
                    "This map was saved by a newer version of Rustforged (map format {}, \
                     this version reads up to {}).",
                    map.format_version, MAP_FORMAT_VERSION
                ))
            } else {
                Ok(map)
            }
        });
    let mut saved_map = match parsed {
        Ok(mut saved_map) => {
            saved_map.normalize();
            saved_map
        }
        Err(error) => {
            return LoadResult {
                path,
                saved_map: None,
                error: Some(error),
                disk_hash,
                relinked: Vec::new(),
                missing: Vec::new(),
                computed: index.into_computed(),
            };
        }
    };

    let mut relinked = Vec::new();
    let mut missing = Vec::new();
    for asset in saved_map.used_assets() {
        match index.resolve(&asset, saved_map.asset_fingerprints.get(&asset)) {
            Some(found) if found == asset => {}
            Some(found) => relinked.push((asset, found)),
            None => {
                let candidate = index.same_name_candidate(&asset);
                missing.push((asset, candidate));
            }
        }
    }
    for (old, new) in &relinked {
        saved_map.retarget_asset(old, new);
    }

    LoadResult {
        path,
        saved_map: Some(saved_map),
        error: None,
        disk_hash,
        relinked,
        missing,
        computed: index.into_computed(),
    }
}

/// Messages sent when a load completes
#[derive(SystemParam)]
pub struct LoadMessages<'w> {
    config: MessageWriter<'w, UpdateLastMapPathRequest>,
    replaced: MessageWriter<'w, MapReplaced>,
}

/// Polls load tasks and handles completion (spawns entities synchronously)
#[allow(clippy::too_many_arguments)]
pub fn poll_load_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut LoadMapTask)>,
    mut async_op: ResMut<AsyncMapOperation>,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
    mut load_error: ResMut<MapLoadError>,
    mut asset_report: ResMut<MapAssetReport>,
    mut disk_change: ResMut<MapChangedOnDisk>,
    mut asset_library: ResMut<AssetLibrary>,
    asset_server: Res<AssetServer>,
    existing_items: Query<Entity, With<PlacedItem>>,
    existing_annotations: Query<Entity, With<AnnotationMarker>>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut messages: LoadMessages,
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = poll_task(&mut task.0) {
            // Clear async state
            async_op.is_loading = false;
            async_op.operation_description = None;
            load_error.message = None;

            // Handle error
            if let Some(error) = result.error {
                load_error.message = Some(error.clone());
                error!("{}", error);
                commands.entity(entity).despawn();
                continue;
            }

            asset_library.merge_fingerprints(result.computed);

            let Some(mut saved_map) = result.saved_map else {
                commands.entity(entity).despawn();
                continue;
            };
            let asset_fingerprints = std::mem::take(&mut saved_map.asset_fingerprints);
            let map_id = saved_map.map_id.take();

            for (old, new) in &result.relinked {
                info!("Relinked asset {} -> {}", old, new);
            }
            for (asset, _) in &result.missing {
                warn!("Asset not in library, using placeholder: {}", asset);
            }

            // Clear existing items
            for existing in existing_items.iter() {
                commands.entity(existing).despawn();
            }

            // Clear existing annotations
            for existing in existing_annotations.iter() {
                commands.entity(existing).despawn();
            }

            spawn_map_contents(&mut commands, &asset_server, &saved_map);

            // Load map data
            *map_data = saved_map.map_data;

            // Load fog of war data
            *fog_data = FogOfWarData::from(saved_map.fog_of_war);

            messages.replaced.write(MapReplaced);
            info!("Map loaded from {:?}", result.path);
            disk_change.clear_for(&result.path);

            // Update current map file and config
            current_map_file.path = Some(result.path.clone());
            messages.config.write(UpdateLastMapPathRequest {
                path: result.path.clone(),
            });

            // A freshly loaded map is clean, unless references were relinked: the new paths
            // only reach the file when the user saves.
            let relinked_any = !result.relinked.is_empty();
            dirty_state.is_dirty = relinked_any;
            // Ignore the spawn wave from loading so the map stays clean.
            dirty_state.suppress_change_detection();

            // Update open maps - check if map is already open or replace current
            let map_name = result
                .path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            // Check if this map is already open (by path)
            let existing_id = open_maps
                .maps
                .iter()
                .find(|(_, m)| m.path.as_ref() == Some(&result.path))
                .map(|(id, _)| *id);

            if let Some(id) = existing_id {
                // Map already open - just switch to it and update state
                if let Some(map) = open_maps.maps.get_mut(&id) {
                    map.map_id = map_id;
                    map.disk_hash = result.disk_hash;
                    map.asset_fingerprints = asset_fingerprints;
                }
                open_maps.active_map_id = Some(id);
            } else {
                // Map not open - replace the current active map entry
                if let Some(active_id) = open_maps.active_map_id {
                    open_maps.maps.remove(&active_id);
                }

                let new_id = open_maps.next_id;
                open_maps.next_id += 1;

                open_maps.maps.insert(
                    new_id,
                    OpenMap {
                        id: new_id,
                        name: map_name.clone(),
                        path: Some(result.path.clone()),
                        stashed_dirty: false,
                        saved_state: None,
                        map_id,
                        disk_hash: result.disk_hash,
                        asset_fingerprints,
                    },
                );
                open_maps.active_map_id = Some(new_id);
            }

            if relinked_any || !result.missing.is_empty() {
                *asset_report = MapAssetReport {
                    show: true,
                    map_name,
                    relinked: result.relinked,
                    missing: result.missing,
                };
            }

            commands.entity(entity).despawn();
        }
    }
}
