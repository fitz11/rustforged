//! Map save system and task polling.

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use futures_lite::future;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::assets::{AssetFingerprint, AssetLibrary, LibraryIndex};
use crate::config::UpdateLastMapPathRequest;
use crate::editor::{DrawnLine, DrawnPath, TextAnnotation};
use crate::map::{FogOfWarData, MapData, PlacedItem, SavedMap};

use super::helpers::saved_by_of;
use super::map_state::capture_current_map_state;
use super::messages::{MapSaveFinished, SaveMapRequest, SaveOutcome};
use super::resources::{
    AsyncMapOperation, CurrentMapFile, MapChangedOnDisk, MapDirtyState, MapSaveError, OpenMaps,
    SaveConflict, SaveConflictDialog, SaveConflictKind, SaveMapTask,
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
    dirty_state: Res<MapDirtyState>,
    mut waiting: Local<Option<SaveMapRequest>>,
) {
    // A request made while another save or load runs waits for it (the latest one wins)
    for event in events.read() {
        *waiting = Some(event.clone());
    }
    if async_op.is_busy() {
        return;
    }
    if let Some(event) = waiting.take() {
        // Placed items already hold library-relative paths, which is what map files store
        let mut saved_map =
            capture_current_map_state(&map_data, &fog_data, &placed_items, &paths, &lines, &texts);
        let used_assets = saved_map.used_assets();
        let mut index = asset_library.index();
        let active = open_maps.active_map();
        let carried = active
            .map(|m| m.asset_fingerprints.clone())
            .unwrap_or_default();

        let path = event.path.clone();

        // Keep the map's id across saves; saving it under a different file makes a copy,
        // which gets its own id.
        let same_file = active.is_some_and(|m| m.path.is_none() || m.path.as_ref() == Some(&path));
        let map_id = active
            .and_then(|m| m.map_id.clone())
            .filter(|_| same_file)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        saved_map.map_id = Some(map_id.clone());
        // A saved map is named after its file
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            saved_map.map_data.name = stem.to_string();
        }
        saved_map.saved_by = crate::fsutil::machine_name();
        // The version of the file this map came from; None for a new file or a copy
        let expected_hash = active
            .filter(|_| same_file)
            .and_then(|m| m.disk_hash.clone());
        let overwrite = event.overwrite;

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

            let conflict = if overwrite {
                None
            } else {
                check_save_conflict(&path, expected_hash.as_deref())
            };
            let written = match conflict {
                Some(_) => Err(None),
                None => write_map(&path, &mut saved_map).map_err(Some),
            };
            SaveResult {
                path,
                success: written.is_ok(),
                error: written.as_ref().err().cloned().flatten(),
                conflict,
                disk_hash: written.ok(),
                map_id,
                asset_fingerprints: fingerprints,
                computed: index.into_computed(),
            }
        });

        commands.spawn(SaveMapTask {
            task,
            map: active.map(|m| m.id).unwrap_or_default(),
            generation: dirty_state.generation,
            ticket: event.ticket,
        });
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

/// Compares the file on disk with the version this map was opened from (`expected_hash`,
/// None for a new file). Returns a conflict when saving would overwrite changes made
/// elsewhere, such as on another machine sharing a synced library.
pub(super) fn check_save_conflict(
    path: &Path,
    expected_hash: Option<&str>,
) -> Option<SaveConflict> {
    let kind = match (std::fs::read(path), expected_hash) {
        (Ok(bytes), Some(expected)) if crate::fsutil::content_hash(&bytes) == expected => {
            return None;
        }
        (Ok(bytes), expected) => {
            let saved_by = saved_by_of(&bytes);
            let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
            let kind = if expected.is_some() {
                SaveConflictKind::Changed
            } else {
                SaveConflictKind::Exists
            };
            return Some(SaveConflict {
                path: path.to_path_buf(),
                kind,
                saved_by,
                modified,
            });
        }
        (Err(e), Some(_)) if e.kind() == std::io::ErrorKind::NotFound => SaveConflictKind::Deleted,
        // Nothing there yet, or unreadable: let the write report any error
        (Err(_), _) => return None,
    };
    Some(SaveConflict {
        path: path.to_path_buf(),
        kind,
        saved_by: None,
        modified: None,
    })
}

/// Writes the map file and returns the content hash of what's now on disk.
pub(super) fn write_map(path: &Path, saved_map: &mut SavedMap) -> Result<String, String> {
    saved_map.sort_for_save();
    // Serialize (could be slow for large maps)
    let json = serde_json::to_string_pretty(saved_map)
        .map_err(|e| format!("Failed to serialize map: {}", e))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create maps folder: {}", e))?;
    }
    // Atomic, and skipped when nothing changed so sync clients have nothing to upload
    let wrote = crate::fsutil::write_if_changed(path, json.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;
    if !wrote {
        info!("Map unchanged on disk, skipped writing {:?}", path);
    }
    Ok(crate::fsutil::content_hash(json.as_bytes()))
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
    mut conflict_dialog: ResMut<SaveConflictDialog>,
    mut disk_change: ResMut<MapChangedOnDisk>,
    mut finished: MessageWriter<MapSaveFinished>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.task)) {
            // Clear async state
            async_op.is_saving = false;
            async_op.operation_description = None;

            asset_library.merge_fingerprints(result.computed);

            if result.success {
                info!("Map saved to {:?}", result.path);
                disk_change.clear_for(&result.path);

                // Clear any previous save error
                save_error.message = None;

                config_events.write(UpdateLastMapPathRequest {
                    path: result.path.clone(),
                });

                // The map that was saved, which may no longer be the active one (or open)
                let is_active = open_maps.active_map_id == Some(task.map);
                if let Some(saved) = open_maps.maps.get_mut(&task.map) {
                    saved.asset_fingerprints = result.asset_fingerprints;
                    saved.map_id = Some(result.map_id);
                    saved.disk_hash = result.disk_hash;
                    saved.path = Some(result.path.clone());
                    saved.name = result
                        .path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                }
                if is_active {
                    current_map_file.path = Some(result.path.clone());
                    // Edits made while the file was being written aren't in it
                    if dirty_state.generation == task.generation {
                        dirty_state.is_dirty = false;
                    }
                }
                finished.write(MapSaveFinished {
                    ticket: task.ticket,
                    outcome: SaveOutcome::Saved,
                });
            } else if let Some(conflict) = result.conflict {
                warn!(
                    "Not saving {:?}: {:?} on disk since it was opened",
                    conflict.path, conflict.kind
                );
                conflict_dialog.conflict = Some(conflict);
                finished.write(MapSaveFinished {
                    ticket: task.ticket,
                    outcome: SaveOutcome::Conflict,
                });
            } else {
                if let Some(error) = result.error {
                    error!("{}", error);
                    // Store error for display to user
                    save_error.message = Some(error);
                }
                finished.write(MapSaveFinished {
                    ticket: task.ticket,
                    outcome: SaveOutcome::Failed,
                });
            }

            commands.entity(entity).despawn();
        }
    }
}
