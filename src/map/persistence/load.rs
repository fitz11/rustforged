//! Map load system and task polling.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use futures_lite::future;
use std::path::PathBuf;

use crate::assets::{AssetLibrary, LibraryIndex, library_asset_path};
use crate::config::UpdateLastMapPathRequest;
use crate::editor::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use crate::map::{FogOfWarData, Layer, MAP_FORMAT_VERSION, MapData, PlacedItem, SavedMap};

use super::helpers::array_to_color;
use super::messages::LoadMapRequest;
use super::resources::{
    AsyncMapOperation, CurrentMapFile, LoadMapTask, MapAssetReport, MapDirtyState, MapLoadError,
    OpenMap, OpenMaps,
};
use super::results::LoadResult;

/// Starts an async load operation (file I/O and asset reference resolution)
pub fn load_map_system(
    mut commands: Commands,
    mut events: MessageReader<LoadMapRequest>,
    mut async_op: ResMut<AsyncMapOperation>,
    asset_library: Res<AssetLibrary>,
) {
    for event in events.read() {
        // Don't start a new load if one is already in progress
        if async_op.is_busy() {
            warn!("Load operation already in progress");
            continue;
        }

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
    let parsed = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))
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
        Ok(saved_map) => saved_map,
        Err(error) => {
            return LoadResult {
                path,
                saved_map: None,
                error: Some(error),
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
            None => missing.push(asset),
        }
    }
    for (old, new) in &relinked {
        saved_map.retarget_asset(old, new);
    }

    LoadResult {
        path,
        saved_map: Some(saved_map),
        error: None,
        relinked,
        missing,
        computed: index.into_computed(),
    }
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
    mut asset_library: ResMut<AssetLibrary>,
    asset_server: Res<AssetServer>,
    existing_items: Query<Entity, With<PlacedItem>>,
    existing_annotations: Query<Entity, With<AnnotationMarker>>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut config_events: MessageWriter<UpdateLastMapPathRequest>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
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

            if !result.computed.is_empty() {
                asset_library.fingerprints.merge(result.computed);
            }

            let Some(mut saved_map) = result.saved_map else {
                commands.entity(entity).despawn();
                continue;
            };
            let asset_fingerprints = std::mem::take(&mut saved_map.asset_fingerprints);
            let map_id = saved_map.map_id.take();

            for (old, new) in &result.relinked {
                info!("Relinked asset {} -> {}", old, new);
            }
            for asset in &result.missing {
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

            // Load map data
            *map_data = saved_map.map_data;

            // Load fog of war data
            *fog_data = FogOfWarData::from(saved_map.fog_of_war);

            // Spawn placed items
            for item in saved_map.placed_items {
                let texture: Handle<Image> =
                    asset_server.load(library_asset_path(&item.asset_path));
                // Clamp z_index to valid range (for migration from old maps with larger ranges)
                let z_index = item.z_index.clamp(0, Layer::max_z_index());
                let z = item.layer.z_base() + z_index as f32;

                // Items on non-player-visible layers (GM, FogOfWar) go to render layer 1 (editor-only)
                let render_layer = if item.layer.is_player_visible() {
                    RenderLayers::layer(0)
                } else {
                    RenderLayers::layer(1)
                };

                commands.spawn((
                    Sprite::from_image(texture),
                    Transform {
                        translation: item.position.extend(z),
                        rotation: Quat::from_rotation_z(item.rotation),
                        scale: item.scale.extend(1.0),
                    },
                    PlacedItem {
                        asset_path: item.asset_path,
                        layer: item.layer,
                        z_index,
                    },
                    render_layer,
                ));
            }

            // Spawn annotations
            let z = Layer::Annotation.z_base();

            for path in saved_map.annotations.paths {
                commands.spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                    DrawnPath {
                        points: path.points,
                        color: array_to_color(path.color),
                        stroke_width: path.stroke_width,
                    },
                    AnnotationMarker,
                ));
            }

            for line in saved_map.annotations.lines {
                commands.spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                    DrawnLine {
                        start: line.start,
                        end: line.end,
                        color: array_to_color(line.color),
                        stroke_width: line.stroke_width,
                    },
                    AnnotationMarker,
                ));
            }

            for text in saved_map.annotations.text_boxes {
                commands.spawn((
                    Transform::from_translation(text.position.extend(z)),
                    TextAnnotation {
                        content: text.content,
                        font_size: text.font_size,
                        color: array_to_color(text.color),
                    },
                    AnnotationMarker,
                ));
            }

            info!("Map loaded from {:?}", result.path);

            // Update current map file and config
            current_map_file.path = Some(result.path.clone());
            config_events.write(UpdateLastMapPathRequest {
                path: result.path.clone(),
            });

            // A freshly loaded map is clean, unless references were relinked: the new paths
            // only reach the file when the user saves.
            let relinked_any = !result.relinked.is_empty();
            dirty_state.is_dirty = relinked_any;
            dirty_state.last_known_item_count = 0; // Will be updated by detection system
            dirty_state.last_known_annotation_count = 0;
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
                    map.is_dirty = relinked_any;
                    map.map_id = map_id;
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
                        is_dirty: relinked_any,
                        saved_state: None,
                        map_id,
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
