//! Map load system and task polling.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use futures_lite::future;
use std::collections::{HashMap, HashSet};

use crate::assets::AssetLibrary;
use crate::config::UpdateLastMapPathRequest;
use crate::editor::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use crate::map::{FogOfWarData, Layer, MapData, PlacedItem, SavedMap};

use super::helpers::array_to_color;
use super::messages::LoadMapRequest;
use super::resources::{
    AsyncMapOperation, CurrentMapFile, LoadMapTask, LoadValidationWarning, MapDirtyState,
    MapLoadError, OpenMap, OpenMaps,
};
use super::results::LoadResult;

/// Starts an async load operation (file I/O only)
pub fn load_map_system(
    mut commands: Commands,
    mut events: MessageReader<LoadMapRequest>,
    mut async_op: ResMut<AsyncMapOperation>,
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

        // Spawn async task for file I/O and parsing
        let task_pool = IoTaskPool::get();
        let task = task_pool.spawn(async move {
            // Read file
            let json = match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    return LoadResult {
                        path,
                        saved_map: None,
                        error: Some(format!("Failed to read file: {}", e)),
                    };
                }
            };

            // Parse JSON
            match serde_json::from_str::<SavedMap>(&json) {
                Ok(saved_map) => LoadResult {
                    path,
                    saved_map: Some(saved_map),
                    error: None,
                },
                Err(e) => LoadResult {
                    path,
                    saved_map: None,
                    error: Some(format!("Failed to parse map file: {}", e)),
                },
            }
        });

        commands.spawn(LoadMapTask(task));
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
    mut load_warning: ResMut<LoadValidationWarning>,
    asset_library: Res<AssetLibrary>,
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

            let Some(saved_map) = result.saved_map else {
                commands.entity(entity).despawn();
                continue;
            };

            // Build lookup maps for multi-strategy path resolution
            let relative_to_bevy = asset_library.build_relative_to_bevy_map();
            let available_bevy_paths: HashSet<&str> = asset_library
                .assets
                .iter()
                .map(|a| a.relative_path.as_str())
                .collect();

            // Resolve each manifest path using 3 strategies:
            // 1. Library-relative match (new format)
            // 2. Exact Bevy-path match (old format, same location)
            // 3. Suffix match (old format, moved library)
            let mut path_mapping: HashMap<String, String> = HashMap::new();
            let mut missing_assets: Vec<String> = Vec::new();

            for saved_path in &saved_map.asset_manifest.assets {
                if let Some(&bevy_path) = relative_to_bevy.get(saved_path.as_str()) {
                    // Strategy 1: library-relative match
                    path_mapping.insert(saved_path.clone(), bevy_path.to_string());
                } else if available_bevy_paths.contains(saved_path.as_str()) {
                    // Strategy 2: exact Bevy-path match (old format, library hasn't moved)
                    path_mapping.insert(saved_path.clone(), saved_path.clone());
                } else {
                    // Strategy 3: suffix match (old format, library has moved)
                    let mut matched = false;
                    for (rel_path, &bevy_path) in &relative_to_bevy {
                        if saved_path.ends_with(rel_path.as_str()) {
                            path_mapping.insert(saved_path.clone(), bevy_path.to_string());
                            matched = true;
                            break;
                        }
                    }
                    if !matched {
                        missing_assets.push(saved_path.clone());
                    }
                }
            }

            if !missing_assets.is_empty() {
                // Show warning dialog with missing assets
                load_warning.show = true;
                load_warning.missing_assets = missing_assets.clone();
                load_warning.map_path = Some(result.path.clone());

                warn!(
                    "Cannot load map {:?}: {} missing assets",
                    result.path,
                    missing_assets.len()
                );
                commands.entity(entity).despawn();
                continue;
            }

            // Rewrite placed item paths to current Bevy-loadable paths
            let mut saved_map = saved_map;
            for item in &mut saved_map.placed_items {
                if let Some(bevy_path) = path_mapping.get(&item.asset_path) {
                    item.asset_path = bevy_path.clone();
                }
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
                let texture: Handle<Image> = asset_server.load(&item.asset_path);
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

            // Clear dirty state (freshly loaded map is clean)
            dirty_state.is_dirty = false;
            dirty_state.last_known_item_count = 0; // Will be updated by detection system
            dirty_state.last_known_annotation_count = 0;

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
                    map.is_dirty = false;
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
                        name: map_name,
                        path: Some(result.path.clone()),
                        is_dirty: false,
                        saved_state: None,
                    },
                );
                open_maps.active_map_id = Some(new_id);
            }

            commands.entity(entity).despawn();
        }
    }
}
