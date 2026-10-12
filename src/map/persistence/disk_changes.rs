//! Notices when the open map's file changes on disk while it's open, for example when a
//! newer version syncs in from another machine.

use bevy::prelude::*;

use crate::assets::MapFilesChanged;

use super::helpers::saved_by_of;
use super::resources::{DiskChangeNotice, MapChangedOnDisk, OpenMaps};

/// System that compares the active map's file with the version it was opened from whenever
/// the library watcher reports map changes.
pub fn detect_open_map_changes(
    mut events: MessageReader<MapFilesChanged>,
    open_maps: Res<OpenMaps>,
    mut disk_change: ResMut<MapChangedOnDisk>,
) {
    let active_path = open_maps.active_map().and_then(|m| m.path.as_ref());
    let mut touched = false;
    for event in events.read() {
        touched |= active_path.is_some_and(|path| event.paths.contains(path));
    }
    let (Some(active), Some(path)) = (open_maps.active_map(), active_path) else {
        return;
    };
    if !touched {
        return;
    }

    match std::fs::read(path) {
        // Our own save, or the file changed back
        Ok(bytes) if active.disk_hash.as_deref() == Some(&crate::fsutil::content_hash(&bytes)) => {
            disk_change.clear_for(path);
        }
        Ok(bytes) => {
            let saved_by = saved_by_of(&bytes);
            info!(
                "Open map changed on disk: {:?} (saved by {:?})",
                path, saved_by
            );
            disk_change.notice = Some(DiskChangeNotice {
                path: path.clone(),
                saved_by,
                deleted: false,
            });
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            info!("Open map was moved or deleted on disk: {:?}", path);
            disk_change.notice = Some(DiskChangeNotice {
                path: path.clone(),
                saved_by: None,
                deleted: true,
            });
        }
        // Mid-sync or locked; the next change event will try again
        Err(_) => {}
    }
}
