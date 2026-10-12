//! Thumbnail loading and registration system.

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use bevy_egui::{EguiTextureHandle, EguiUserTextures};
use std::path::PathBuf;

use crate::assets::{AssetLibrary, LibraryCache, ThumbnailCache, load_thumbnail};
use crate::constants::MAX_THUMBNAILS_IN_FLIGHT;

use super::state::AssetBrowserState;
use crate::common::poll_task;

/// System that loads thumbnails and registers them with egui.
///
/// Only rows the asset list showed last frame get thumbnails, so a big library isn't read
/// in full (which, for online-only files in a synced folder, would download them all).
/// Reading and rendering happen in the background, a few at a time, so large images don't
/// stall the editor or the player view.
pub fn load_and_register_thumbnails(
    library: Res<AssetLibrary>,
    library_cache: Res<LibraryCache>,
    browser_state: Res<AssetBrowserState>,
    mut thumbnail_cache: ResMut<ThumbnailCache>,
    mut images: ResMut<Assets<Image>>,
    mut egui_textures: ResMut<EguiUserTextures>,
) {
    // Files that changed on disk since their thumbnail was made (an edit, or a sync client
    // finishing a download) get a fresh one
    let stale: Vec<PathBuf> = library
        .assets
        .iter()
        .filter(|a| thumbnail_cache.is_stale(a))
        .map(|a| a.full_path.clone())
        .collect();
    for path in stale {
        thumbnail_cache.invalidate(&path, &mut egui_textures);
    }

    // Collect finished thumbnails
    let finished: Vec<(PathBuf, Option<Image>)> = thumbnail_cache
        .pending
        .iter_mut()
        .filter_map(|(path, task)| poll_task(task).map(|image| (path.clone(), image)))
        .collect();
    for (path, image) in finished {
        thumbnail_cache.pending.remove(&path);
        match image {
            Some(image) => {
                let handle = images.add(image);
                thumbnail_cache.thumbnails.insert(path, handle);
            }
            None => {
                thumbnail_cache.failed.insert(path, ());
            }
        }
    }

    // Start the visible rows that don't have one yet
    for path in &browser_state.visible_assets {
        if thumbnail_cache.pending.len() >= MAX_THUMBNAILS_IN_FLIGHT {
            break;
        }
        if thumbnail_cache.thumbnails.contains_key(path)
            || thumbnail_cache.has_failed(path)
            || thumbnail_cache.pending.contains_key(path)
        {
            continue;
        }
        let Some(asset) = library.assets.iter().find(|a| a.full_path == *path) else {
            continue;
        };
        thumbnail_cache.stamps.insert(path.clone(), asset.stamp);
        let asset = asset.clone();
        let cache_dir = library_cache.dir.clone();
        // Cached thumbnails avoid decoding (and, for online-only files, downloading) the image
        let task =
            IoTaskPool::get().spawn(async move { load_thumbnail(&asset, cache_dir.as_deref()) });
        thumbnail_cache.pending.insert(path.clone(), task);
    }

    // Register any thumbnails that don't have texture IDs yet
    let to_register: Vec<PathBuf> = thumbnail_cache
        .thumbnails
        .keys()
        .filter(|path| !thumbnail_cache.texture_ids.contains_key(*path))
        .cloned()
        .collect();

    for path in to_register {
        if let Some(handle) = thumbnail_cache.thumbnails.get(&path) {
            let texture_id = egui_textures.add_image(EguiTextureHandle::Weak(handle.id()));
            thumbnail_cache.texture_ids.insert(path, texture_id);
        }
    }
}
