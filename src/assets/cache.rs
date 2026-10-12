//! Machine-local caches for the open library: content fingerprints and thumbnails.
//!
//! They live in the user's cache directory, never inside the library, so they don't sync
//! and each machine keeps its own. Entries are keyed by file size and modification time, so
//! a file that changed (or was re-downloaded by a sync client) is simply read again. Without
//! them, every launch would decode every visible image in full, which also re-downloads
//! online-only files. Everything here can be deleted; it's rebuilt as needed.

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::fingerprint::{FileStamp, FingerprintCache};
use super::library::{AssetLibrary, LibraryAsset, render_thumbnail, thumbnail_to_image};
use super::manifest::LibraryMetadata;
use crate::common::poll_task;

const FINGERPRINTS_FILE: &str = "fingerprints.json";
const THUMBNAILS_DIR: &str = "thumbnails";

/// Resource pointing at the open library's cache folder.
#[derive(Resource, Default)]
pub struct LibraryCache {
    /// None if no cache directory is available on this platform
    pub dir: Option<PathBuf>,
}

/// Names the cache folder for a library: its manifest id, which is the same wherever the
/// library is mounted, or a hash of its location for folders without one.
fn cache_key(library_path: &Path, metadata: &LibraryMetadata) -> String {
    // The id comes from a synced file, so keep only characters that are safe in a path
    let id: String = metadata
        .id
        .iter()
        .flat_map(|id| id.chars())
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(64)
        .collect();
    if !id.is_empty() {
        return id;
    }
    location_key(library_path)
}

/// Cache folder name for a library without an id, from where it is.
fn location_key(library_path: &Path) -> String {
    let location = library_path
        .canonicalize()
        .unwrap_or_else(|_| library_path.to_path_buf());
    format!(
        "path-{}",
        &crate::fsutil::content_hash(location.to_string_lossy().as_bytes())[..16]
    )
}

/// Cache file for an asset's thumbnail, or None when its modification time is unknown (so a
/// cached thumbnail couldn't be checked for staleness).
fn thumbnail_file(dir: &Path, path: &str, stamp: FileStamp) -> Option<PathBuf> {
    let modified = stamp.modified_ns?;
    let key = crate::fsutil::content_hash(format!("{path}\0{}\0{modified}", stamp.size).as_bytes());
    Some(dir.join(THUMBNAILS_DIR).join(format!("{key}.png")))
}

/// Loads an asset's thumbnail from the cache, or renders it from the image and caches it.
pub fn load_thumbnail(asset: &LibraryAsset, cache_dir: Option<&Path>) -> Option<Image> {
    let cached = cache_dir.and_then(|dir| thumbnail_file(dir, &asset.path, asset.stamp));
    if let Some(file) = &cached
        && let Ok(image) = image::open(file)
    {
        return Some(thumbnail_to_image(image.to_rgba8()));
    }

    let rgba = render_thumbnail(&asset.full_path)?;
    if let Some(file) = cached {
        let mut png = Vec::new();
        let written = rgba
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|e| e.to_string())
            .and_then(|()| {
                std::fs::create_dir_all(file.parent().unwrap_or(Path::new(".")))
                    .and_then(|()| crate::fsutil::write_atomic(&file, &png))
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = written {
            debug!("Couldn't cache thumbnail for {}: {}", asset.path, e);
        }
    }
    Some(thumbnail_to_image(rgba))
}

/// Deletes cached thumbnails that no current asset uses (files that changed or are gone).
fn prune_thumbnails(dir: &Path, keep: &HashSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir.join(THUMBNAILS_DIR)) else {
        return;
    };
    for entry in entries.flatten() {
        if !keep.contains(&entry.path()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn read_fingerprints(dir: &Path) -> Option<FingerprintCache> {
    let json = std::fs::read(dir.join(FINGERPRINTS_FILE)).ok()?;
    serde_json::from_slice(&json)
        .map_err(|e| warn!("Ignoring unreadable fingerprint cache: {}", e))
        .ok()
}

/// System that points the cache at the open library when it changes: loads its saved
/// fingerprints and clears out thumbnails of files that changed since last time.
pub fn open_library_cache(
    mut library: ResMut<AssetLibrary>,
    mut cache: ResMut<LibraryCache>,
    mut last_opened: Local<Option<(PathBuf, Option<String>)>>,
) {
    let current = (library.library_path.clone(), library.metadata.id.clone());
    if last_opened.as_ref() == Some(&current) {
        return;
    }
    *last_opened = Some(current);

    cache.dir = crate::paths::cache_dir()
        .map(|dir| dir.join(cache_key(&library.library_path, &library.metadata)));
    let Some(dir) = cache.dir.clone() else {
        return;
    };
    debug!("Library cache: {:?}", dir);

    if let Some(saved) = read_fingerprints(&dir) {
        library.load_saved_fingerprints(saved);
    }

    let keep: HashSet<PathBuf> = library
        .assets
        .iter()
        .filter_map(|a| thumbnail_file(&dir, &a.path, a.stamp))
        .collect();
    // A library that got an id (when created or renamed) used to be cached under a key
    // from its location; that folder is now unused
    let old_dir = crate::paths::cache_dir()
        .map(|root| root.join(location_key(&library.library_path)))
        .filter(|old| *old != dir);
    IoTaskPool::get()
        .spawn(async move {
            prune_thumbnails(&dir, &keep);
            if let Some(old) = old_dir
                && old.is_dir()
                && let Err(e) = std::fs::remove_dir_all(&old)
            {
                debug!("Couldn't remove old cache {:?}: {}", old, e);
            }
        })
        .detach();
}

/// System that writes the fingerprint cache after save/load tasks added to it. One write at
/// a time: a newer one waits for the last to finish, so an older snapshot can't land last.
pub fn save_fingerprint_cache(
    library: Res<AssetLibrary>,
    cache: Res<LibraryCache>,
    mut saved_revision: Local<u64>,
    mut writing: Local<Option<bevy::tasks::Task<()>>>,
) {
    if let Some(task) = writing.as_mut() {
        if poll_task(task).is_none() {
            return;
        }
        *writing = None;
    }
    if *saved_revision == library.fingerprint_revision() {
        return;
    }
    *saved_revision = library.fingerprint_revision();
    let Some(dir) = cache.dir.clone() else {
        return;
    };

    let mut fingerprints = library.fingerprints.clone();
    let current: HashSet<&str> = library.assets.iter().map(|a| a.path.as_str()).collect();
    fingerprints.retain_paths(|path| current.contains(path));
    *writing = Some(IoTaskPool::get().spawn(async move {
        let written = serde_json::to_vec(&fingerprints)
            .map_err(|e| e.to_string())
            .and_then(|json| {
                std::fs::create_dir_all(&dir)
                    .and_then(|()| crate::fsutil::write_atomic(&dir.join(FINGERPRINTS_FILE), &json))
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = written {
            warn!("Couldn't save fingerprint cache: {}", e);
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(id: Option<&str>) -> LibraryMetadata {
        LibraryMetadata {
            id: id.map(str::to_string),
            ..LibraryMetadata::default()
        }
    }

    #[test]
    fn test_cache_key_uses_library_id_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let id = "0b7e5c5e-8d1f-4c33-9a57-2f2f4b1f9e10";
        assert_eq!(cache_key(dir.path(), &metadata(Some(id))), id);
    }

    #[test]
    fn test_cache_key_strips_unsafe_id_characters() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(cache_key(dir.path(), &metadata(Some("../../etc"))), "etc");
        // Nothing usable left: fall back to the location
        assert!(cache_key(dir.path(), &metadata(Some("../"))).starts_with("path-"));
    }

    #[test]
    fn test_cache_key_without_id_depends_on_location() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let key = cache_key(a.path(), &metadata(None));
        assert!(key.starts_with("path-"));
        assert_eq!(key, cache_key(a.path(), &metadata(None)));
        assert_ne!(key, cache_key(b.path(), &metadata(None)));
    }

    #[test]
    fn test_thumbnail_file_changes_with_stamp() {
        let dir = Path::new("/cache");
        let stamp = FileStamp {
            size: 10,
            modified_ns: Some(5),
        };
        let first = thumbnail_file(dir, "tokens/orc.png", stamp).unwrap();
        let touched = FileStamp {
            modified_ns: Some(6),
            ..stamp
        };
        assert_ne!(
            first,
            thumbnail_file(dir, "tokens/orc.png", touched).unwrap()
        );
        assert_ne!(
            first,
            thumbnail_file(dir, "tokens/goblin.png", stamp).unwrap()
        );
        let unknown = FileStamp {
            modified_ns: None,
            ..stamp
        };
        assert!(thumbnail_file(dir, "tokens/orc.png", unknown).is_none());
    }

    #[test]
    fn test_load_thumbnail_caches_rendered_thumbnail() {
        let library = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let full_path = library.path().join("orc.png");
        image::RgbaImage::from_pixel(64, 32, image::Rgba([200, 30, 30, 255]))
            .save(&full_path)
            .unwrap();
        let asset = LibraryAsset {
            name: "orc".into(),
            path: "orc.png".into(),
            folder_path: String::new(),
            extension: "png".into(),
            stamp: FileStamp::of_metadata(&std::fs::metadata(&full_path).unwrap()),
            full_path: full_path.clone(),
        };

        let rendered = load_thumbnail(&asset, Some(cache.path())).unwrap();
        let cached_file = thumbnail_file(cache.path(), &asset.path, asset.stamp).unwrap();
        assert!(cached_file.exists());

        // With the source image gone, the thumbnail still comes from the cache
        std::fs::remove_file(&full_path).unwrap();
        let from_cache = load_thumbnail(&asset, Some(cache.path())).unwrap();
        assert_eq!(rendered.size(), from_cache.size());
    }

    #[test]
    fn test_thumbnail_goes_stale_when_file_stamp_changes() {
        let mut thumbnails = super::super::ThumbnailCache::default();
        let mut asset = LibraryAsset {
            name: "orc".into(),
            path: "orc.png".into(),
            folder_path: String::new(),
            extension: "png".into(),
            full_path: PathBuf::from("/lib/orc.png"),
            stamp: FileStamp {
                size: 10,
                modified_ns: Some(1),
            },
        };
        assert!(!thumbnails.is_stale(&asset));
        thumbnails
            .stamps
            .insert(asset.full_path.clone(), asset.stamp);
        assert!(!thumbnails.is_stale(&asset));

        asset.stamp.modified_ns = Some(2);
        assert!(thumbnails.is_stale(&asset));
    }

    #[test]
    fn test_prune_thumbnails_keeps_only_current() {
        let cache = tempfile::tempdir().unwrap();
        let thumbs = cache.path().join(THUMBNAILS_DIR);
        std::fs::create_dir_all(&thumbs).unwrap();
        std::fs::write(thumbs.join("keep.png"), "k").unwrap();
        std::fs::write(thumbs.join("stale.png"), "s").unwrap();

        prune_thumbnails(cache.path(), &HashSet::from([thumbs.join("keep.png")]));

        assert!(thumbs.join("keep.png").exists());
        assert!(!thumbs.join("stale.png").exists());
    }
}
