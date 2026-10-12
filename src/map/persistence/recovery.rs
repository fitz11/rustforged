//! Autosave to a recovery folder, and offering it back after a crash.
//!
//! While a map has unsaved changes, it's written every minute to the app's recovery folder,
//! never over the user's own file (which would also defeat save-conflict detection in a
//! synced library). A recovery file is removed once its map is saved or closed, and the
//! whole session's files when the app quits normally; anything left at startup is from a
//! session that didn't quit, and is offered back.

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::common::poll_task;
use crate::map::{FogOfWarData, MapData, SavedMap};

use super::map_state::{MapContents, MapEntities, spawn_map_contents};
use super::messages::{MapReplaced, MapSaveFinished, SaveOutcome};
use super::resources::{CurrentMapFile, MapDirtyState, OpenMap, OpenMaps};

/// Seconds between autosaves of a map with unsaved changes
const AUTOSAVE_SECONDS: f64 = 60.0;

/// A recovery file: the map, and where it came from
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryFile {
    /// The file the map was opened from or last saved to, if any
    pub original_path: Option<PathBuf>,
    /// Its name in the open-maps list
    pub name: String,
    /// The content hash of `original_path` when the map was opened or last saved, so
    /// saving the recovered map still notices changes made to that file since
    pub base_disk_hash: Option<String>,
    /// When this was written (local time, for the user)
    pub saved_at: String,
    pub map: SavedMap,
}

/// Autosave bookkeeping
#[derive(Resource)]
pub struct Autosave {
    /// Identifies this run of the app in recovery file names
    session: String,
    /// The edit count last written, per open map id
    written: bevy::platform::collections::HashMap<u64, u64>,
    /// When the last autosave was written
    last_write: f64,
    writing: Option<Task<()>>,
}

impl Default for Autosave {
    fn default() -> Self {
        Self {
            session: uuid::Uuid::new_v4().to_string(),
            written: Default::default(),
            last_write: 0.0,
            writing: None,
        }
    }
}

impl Autosave {
    fn file_for(&self, dir: &Path, map: u64) -> PathBuf {
        dir.join(format!("{}-{}.json", self.session, map))
    }
}

/// Recovery files found at startup, offered to the user one at a time
#[derive(Resource, Default)]
pub struct RecoveryOffer {
    pub files: Vec<(PathBuf, RecoveryFile)>,
}

/// Asks to open a recovery file as the map being edited
#[derive(Message)]
pub struct RecoverMapRequest {
    pub path: PathBuf,
}

/// Writes the active map to the recovery folder every [`AUTOSAVE_SECONDS`] while it has
/// changes that aren't in its file or in the last recovery file.
#[allow(clippy::too_many_arguments)]
pub fn autosave_recovery_file(
    time: Res<Time>,
    mut autosave: ResMut<Autosave>,
    dirty_state: Res<MapDirtyState>,
    open_maps: Res<OpenMaps>,
    map_data: Res<MapData>,
    fog_data: Res<FogOfWarData>,
    contents: MapContents,
) {
    if let Some(task) = autosave.writing.as_mut() {
        if poll_task(task).is_none() {
            return;
        }
        autosave.writing = None;
    }
    let now = time.elapsed_secs_f64();
    if now - autosave.last_write < AUTOSAVE_SECONDS || !dirty_state.is_dirty {
        return;
    }
    let (Some(active), Some(dir)) = (open_maps.active_map(), crate::paths::recovery_dir()) else {
        return;
    };
    if autosave.written.get(&active.id) == Some(&dirty_state.generation) {
        return;
    }

    let recovery = RecoveryFile {
        original_path: active.path.clone(),
        name: active.name.clone(),
        base_disk_hash: active.disk_hash.clone(),
        saved_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        map: contents.capture(&map_data, &fog_data),
    };
    let file = autosave.file_for(&dir, active.id);
    autosave.written.insert(active.id, dirty_state.generation);
    autosave.last_write = now;
    autosave.writing = Some(IoTaskPool::get().spawn(async move {
        let written = serde_json::to_vec(&recovery)
            .map_err(|e| e.to_string())
            .and_then(|json| {
                std::fs::create_dir_all(&dir)
                    .and_then(|()| crate::fsutil::write_atomic(&file, &json))
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = written {
            warn!("Couldn't write the recovery file: {}", e);
        }
    }));
}

/// Removes recovery files that are no longer needed: the saved map's, and those of maps
/// that were closed or replaced.
pub fn remove_unneeded_recovery_files(
    mut saves: MessageReader<MapSaveFinished>,
    mut replaced: MessageReader<MapReplaced>,
    mut autosave: ResMut<Autosave>,
    open_maps: Res<OpenMaps>,
) {
    let saved = saves.read().any(|s| s.outcome == SaveOutcome::Saved);
    let map_changed = replaced.read().count() > 0;
    if !saved && !map_changed {
        return;
    }
    let Some(dir) = crate::paths::recovery_dir() else {
        return;
    };
    let gone: Vec<u64> = autosave
        .written
        .keys()
        .copied()
        .filter(|id| {
            !open_maps.maps.contains_key(id) || (saved && open_maps.active_map_id == Some(*id))
        })
        .collect();
    for id in gone {
        autosave.written.remove(&id);
        let _ = std::fs::remove_file(autosave.file_for(&dir, id));
    }
}

/// When the app quits normally, its recovery files aren't needed (unsaved changes were
/// either saved or discarded on purpose).
pub fn remove_recovery_files_on_exit(mut exits: MessageReader<AppExit>, autosave: Res<Autosave>) {
    if exits.read().count() == 0 {
        return;
    }
    if let Some(dir) = crate::paths::recovery_dir() {
        for id in autosave.written.keys() {
            let _ = std::fs::remove_file(autosave.file_for(&dir, *id));
        }
    }
}

/// Startup: finds recovery files left by a session that didn't quit normally.
pub fn find_recovery_files(mut offer: ResMut<RecoveryOffer>) {
    let Some(dir) = crate::paths::recovery_dir() else {
        return;
    };
    offer.files = read_recovery_files(&dir);
}

/// The readable recovery files in `dir`, newest first. Unreadable ones are left alone.
fn read_recovery_files(dir: &Path) -> Vec<(PathBuf, RecoveryFile)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter_map(|path| {
            let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            let json = std::fs::read(&path).ok()?;
            let file = serde_json::from_slice::<RecoveryFile>(&json)
                .map_err(|e| warn!("Ignoring unreadable recovery file {:?}: {}", path, e))
                .ok()?;
            Some((modified, path, file))
        })
        .collect();
    files.sort_by_key(|(modified, _, _)| std::cmp::Reverse(*modified));
    files
        .into_iter()
        .map(|(_, path, file)| (path, file))
        .collect()
}

/// Opens a recovery file in place of the map being edited: unsaved, with its original file
/// as where Save goes. The recovery file is removed; autosave writes a new one.
#[allow(clippy::too_many_arguments)]
pub fn recover_map_system(
    mut commands: Commands,
    mut requests: MessageReader<RecoverMapRequest>,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
    mut open_maps: ResMut<OpenMaps>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut dirty_state: ResMut<MapDirtyState>,
    asset_server: Res<AssetServer>,
    existing: MapEntities,
    mut replaced: MessageWriter<MapReplaced>,
) {
    for request in requests.read() {
        let file = match std::fs::read(&request.path)
            .map_err(|e| e.to_string())
            .and_then(|json| {
                serde_json::from_slice::<RecoveryFile>(&json).map_err(|e| e.to_string())
            }) {
            Ok(file) => file,
            Err(e) => {
                warn!("Couldn't recover {:?}: {}", request.path, e);
                continue;
            }
        };

        for entity in &existing {
            commands.entity(entity).despawn();
        }
        let mut map = file.map;
        map.normalize();
        spawn_map_contents(&mut commands, &asset_server, &map);
        *map_data = map.map_data;
        *fog_data = FogOfWarData::from(map.fog_of_war);

        if let Some(active) = open_maps.active_map_id {
            open_maps.maps.remove(&active);
        }
        let id = open_maps.next_id;
        open_maps.next_id += 1;
        open_maps.maps.insert(
            id,
            OpenMap {
                id,
                name: file.name,
                path: file.original_path.clone(),
                stashed_dirty: false,
                saved_state: None,
                map_id: map.map_id,
                disk_hash: file.base_disk_hash,
                asset_fingerprints: map.asset_fingerprints,
                player_view: None,
            },
        );
        open_maps.active_map_id = Some(id);
        current_map_file.path = file.original_path;
        dirty_state.mark_dirty();
        replaced.write(MapReplaced);
        let _ = std::fs::remove_file(&request.path);
        info!("Recovered unsaved changes from {:?}", request.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_files_round_trip_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        for (name, map_name) in [("a.json", "Older"), ("b.json", "Newer")] {
            let file = RecoveryFile {
                original_path: Some(PathBuf::from("maps/cave.json")),
                name: map_name.to_string(),
                base_disk_hash: Some("hash".to_string()),
                saved_at: "2026-10-12 10:00".to_string(),
                map: SavedMap {
                    format_version: crate::map::MAP_FORMAT_VERSION,
                    map_id: None,
                    saved_by: None,
                    map_data: MapData::default(),
                    placed_items: vec![],
                    annotations: Default::default(),
                    fog_of_war: Default::default(),
                    asset_fingerprints: Default::default(),
                },
            };
            std::fs::write(dir.path().join(name), serde_json::to_vec(&file).unwrap()).unwrap();
            // Distinct modification times
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        std::fs::write(dir.path().join("broken.json"), "{").unwrap();

        let files = read_recovery_files(dir.path());

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].1.name, "Newer");
        assert_eq!(
            files[1].1.original_path,
            Some(PathBuf::from("maps/cave.json"))
        );
    }
}
