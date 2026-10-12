//! Watches the open library folder for changes made outside the app: files arriving from
//! a sync client, edits in an image editor, renames in a file manager.
//!
//! Changes are debounced, then:
//! - asset changes rescan the library (which also restores placeholders whose file arrived)
//!   and reload changed images, so placed items and thumbnails update live;
//! - map changes are announced with [`MapFilesChanged`] so the map list and open maps can
//!   react;
//! - a changed library manifest reloads the library name.
//!
//! Sync clients' temp and bookkeeping files are ignored. If the folder can't be watched,
//! the library still works and the user refreshes by hand.

use bevy::prelude::*;
use bevy_egui::EguiUserTextures;
use notify_debouncer_full::notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use super::manifest::{LIBRARY_MANIFEST_FILE, load_library_metadata};
use super::{AssetLibrary, RefreshAssetLibrary, ThumbnailCache, library_asset_path};

/// How long a path must be quiet before its change is reported. Sync clients often write
/// a file in several steps.
const DEBOUNCE: Duration = Duration::from_millis(750);

/// Message sent when map files in the library's maps/ folder changed on disk.
#[derive(Message)]
pub struct MapFilesChanged {
    /// Full paths (library path joined with the relative path) that changed
    pub paths: Vec<PathBuf>,
}

struct WatchState {
    root: PathBuf,
    /// The root as the OS reports it, which may differ (symlinks, macOS /private)
    canonical_root: Option<PathBuf>,
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    events: Receiver<DebounceEventResult>,
}

/// Resource owning the file watcher for the open library.
#[derive(Resource, Default)]
pub struct LibraryWatcher {
    state: Mutex<Option<WatchState>>,
}

fn start_watching(root: &Path) -> notify_debouncer_full::notify::Result<WatchState> {
    let (sender, events) = channel();
    let mut debouncer = new_debouncer(DEBOUNCE, None, sender)?;
    debouncer.watch(root, RecursiveMode::Recursive)?;
    Ok(WatchState {
        root: root.to_path_buf(),
        canonical_root: root.canonicalize().ok(),
        _debouncer: debouncer,
        events,
    })
}

/// System that (re)starts the watcher whenever a different library is opened.
pub fn watch_library_root(library: Res<AssetLibrary>, watcher: Res<LibraryWatcher>) {
    let mut state = watcher.state.lock().unwrap_or_else(|e| e.into_inner());
    if state
        .as_ref()
        .is_some_and(|s| s.root == library.library_path)
    {
        return;
    }
    if state.is_none() && !library.library_path.exists() {
        return;
    }

    *state = match start_watching(&library.library_path) {
        Ok(watch) => {
            info!("Watching library {:?} for changes", library.library_path);
            Some(watch)
        }
        Err(e) => {
            warn!(
                "Can't watch library {:?} for changes ({}); use Refresh to pick up changes",
                library.library_path, e
            );
            None
        }
    };
}

/// Files sync clients and editors create while working: temp files, lock files, partial
/// downloads, version folders. Changes to them aren't library changes.
fn is_sync_noise(relative: &Path) -> bool {
    let hidden = relative
        .components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with('.'));
    let name = relative
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    hidden
        || name.starts_with("~$")
        || name.ends_with('~')
        || [
            ".tmp",
            ".part",
            ".partial",
            ".crdownload",
            ".download",
            ".swp",
        ]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// What a batch of filesystem events means for the library.
#[derive(Debug, Default, PartialEq)]
struct LibraryChanges {
    /// Library-relative paths of changed files or folders outside maps/
    assets: BTreeSet<String>,
    /// Library-relative paths of changed files inside maps/
    maps: BTreeSet<String>,
    manifest: bool,
}

impl LibraryChanges {
    fn add(&mut self, relative: &Path) {
        if relative.as_os_str().is_empty() || is_sync_noise(relative) {
            return;
        }
        let path = relative.to_string_lossy().replace('\\', "/");
        if path == LIBRARY_MANIFEST_FILE {
            self.manifest = true;
        } else if path == "maps" || path.starts_with("maps/") {
            self.maps.insert(path);
        } else {
            self.assets.insert(path);
        }
    }

    fn is_empty(&self) -> bool {
        self.assets.is_empty() && self.maps.is_empty() && !self.manifest
    }
}

impl WatchState {
    fn relative<'a>(&self, path: &'a Path) -> Option<&'a Path> {
        path.strip_prefix(&self.root).ok().or_else(|| {
            self.canonical_root
                .as_ref()
                .and_then(|root| path.strip_prefix(root).ok())
        })
    }

    fn drain(&self) -> LibraryChanges {
        let mut changes = LibraryChanges::default();
        while let Ok(result) = self.events.try_recv() {
            match result {
                Ok(events) => {
                    for event in events {
                        if matches!(event.kind, EventKind::Access(_)) {
                            continue;
                        }
                        for path in &event.paths {
                            if let Some(relative) = self.relative(path) {
                                changes.add(relative);
                            }
                        }
                    }
                }
                Err(errors) => {
                    for e in errors {
                        warn!("Library watcher error: {}", e);
                    }
                }
            }
        }
        changes
    }
}

/// System that applies changes reported by the watcher.
pub fn process_library_changes(
    watcher: Res<LibraryWatcher>,
    mut library: ResMut<AssetLibrary>,
    asset_server: Res<AssetServer>,
    mut thumbnails: ResMut<ThumbnailCache>,
    mut egui_textures: ResMut<EguiUserTextures>,
    mut refresh_events: MessageWriter<RefreshAssetLibrary>,
    mut map_events: MessageWriter<MapFilesChanged>,
) {
    let changes = {
        let state = watcher.state.lock().unwrap_or_else(|e| e.into_inner());
        match state.as_ref() {
            Some(state) if state.root == library.library_path => state.drain(),
            _ => return,
        }
    };
    if changes.is_empty() {
        return;
    }

    if !changes.assets.is_empty() {
        debug!("Library assets changed on disk: {:?}", changes.assets);
        for path in &changes.assets {
            // Reloads only what's loaded, so placed items pick up edited images
            asset_server.reload(library_asset_path(path));
            thumbnails.invalidate(&library.library_path.join(path), &mut egui_textures);
        }
        refresh_events.write(RefreshAssetLibrary);
    }

    if !changes.maps.is_empty() {
        debug!("Map files changed on disk: {:?}", changes.maps);
        map_events.write(MapFilesChanged {
            paths: changes
                .maps
                .iter()
                .map(|p| library.library_path.join(p))
                .collect(),
        });
    }

    if changes.manifest {
        let metadata = load_library_metadata(&library.library_path);
        if metadata.name != library.metadata.name || metadata.id != library.metadata.id {
            info!("Library manifest changed on disk: now '{}'", metadata.name);
            library.metadata = metadata;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changes(paths: &[&str]) -> LibraryChanges {
        let mut changes = LibraryChanges::default();
        for path in paths {
            changes.add(Path::new(path));
        }
        changes
    }

    #[test]
    fn test_sync_client_files_are_ignored() {
        for noise in [
            ".rustforged-a1b2.tmp",
            "tokens/.~lock.orc.png#",
            ".stversions/orc~20260101.png",
            "tokens/.syncthing.orc.png.tmp",
            ".dropbox.cache/x",
            "tokens/orc.png.part",
            "tokens/orc.png.crdownload",
            "tokens/~$draft.png",
            "maps/cave.json~",
        ] {
            assert!(changes(&[noise]).is_empty(), "{noise} should be ignored");
        }
    }

    #[test]
    fn test_changes_are_sorted_into_assets_maps_and_manifest() {
        let changes = changes(&[
            "tokens/orc.png",
            "tokens",
            "maps/cave.json",
            "maps/campaign/town.json",
            LIBRARY_MANIFEST_FILE,
        ]);

        assert_eq!(
            changes.assets.iter().collect::<Vec<_>>(),
            ["tokens", "tokens/orc.png"]
        );
        assert_eq!(
            changes.maps.iter().collect::<Vec<_>>(),
            ["maps/campaign/town.json", "maps/cave.json"]
        );
        assert!(changes.manifest);
    }

    #[test]
    fn test_watcher_reports_new_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("tokens")).unwrap();
        let watch = start_watching(dir.path()).unwrap();

        std::fs::write(dir.path().join("tokens/orc.png"), "orc").unwrap();
        std::fs::write(dir.path().join("tokens/.orc.png.tmp"), "noise").unwrap();

        let mut seen = LibraryChanges::default();
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(50));
            let batch = watch.drain();
            seen.assets.extend(batch.assets);
            if seen.assets.contains("tokens/orc.png") {
                break;
            }
        }
        assert!(seen.assets.contains("tokens/orc.png"));
        assert!(!seen.assets.iter().any(|p| p.contains(".orc.png.tmp")));
    }
}
