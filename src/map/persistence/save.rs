//! Map save system and task polling.

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use futures_lite::future;

use crate::config::UpdateLastMapPathRequest;
use crate::editor::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use crate::map::{
    AssetManifest, FogOfWarData, MapData, PlacedItem, SavedAnnotations, SavedFogOfWar, SavedLine,
    SavedMap, SavedPath, SavedPlacedItem, SavedTextBox,
};

use super::helpers::color_to_array;
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
) {
    for event in events.read() {
        // Don't start a new save if one is already in progress
        if async_op.is_busy() {
            warn!("Save operation already in progress");
            continue;
        }

        let items: Vec<SavedPlacedItem> = placed_items
            .iter()
            .map(|(item, transform)| SavedPlacedItem::from_entity(item, transform))
            .collect();

        // Collect annotations
        let saved_paths: Vec<SavedPath> = paths
            .iter()
            .map(|p| SavedPath {
                points: p.points.clone(),
                color: color_to_array(p.color),
                stroke_width: p.stroke_width,
            })
            .collect();

        let saved_lines: Vec<SavedLine> = lines
            .iter()
            .map(|l| SavedLine {
                start: l.start,
                end: l.end,
                color: color_to_array(l.color),
                stroke_width: l.stroke_width,
            })
            .collect();

        let saved_texts: Vec<SavedTextBox> = texts
            .iter()
            .map(|(transform, t)| SavedTextBox {
                position: transform.translation.truncate(),
                content: t.content.clone(),
                font_size: t.font_size,
                color: color_to_array(t.color),
            })
            .collect();

        // Build asset manifest from placed items
        let asset_manifest = AssetManifest::from_items(items.iter());

        let saved_map = SavedMap {
            asset_manifest,
            map_data: map_data.clone(),
            placed_items: items,
            annotations: SavedAnnotations {
                paths: saved_paths,
                lines: saved_lines,
                text_boxes: saved_texts,
            },
            fog_of_war: SavedFogOfWar::from(&*fog_data),
        };

        let path = event.path.clone();
        let map_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map")
            .to_string();

        // Mark as saving
        async_op.is_saving = true;
        async_op.operation_description = Some(format!("Saving {}...", map_name));

        // Spawn async task for file I/O
        let task_pool = IoTaskPool::get();
        let task = task_pool.spawn(async move {
            // Serialize (could be slow for large maps)
            match serde_json::to_string_pretty(&saved_map) {
                Ok(json) => {
                    // Write to file
                    if let Err(e) = std::fs::write(&path, json) {
                        SaveResult {
                            path,
                            success: false,
                            error: Some(format!("Failed to write file: {}", e)),
                        }
                    } else {
                        SaveResult {
                            path,
                            success: true,
                            error: None,
                        }
                    }
                }
                Err(e) => SaveResult {
                    path,
                    success: false,
                    error: Some(format!("Failed to serialize map: {}", e)),
                },
            }
        });

        commands.spawn(SaveMapTask(task));
    }
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
    placed_items: Query<Entity, With<PlacedItem>>,
    annotations: Query<Entity, With<AnnotationMarker>>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            // Clear async state
            async_op.is_saving = false;
            async_op.operation_description = None;

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
