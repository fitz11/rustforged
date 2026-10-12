//! Thumbnail loading and registration system.

use bevy::prelude::*;
use bevy_egui::{EguiTextureHandle, EguiUserTextures};
use std::path::PathBuf;

use crate::assets::{AssetLibrary, LibraryAsset, LibraryCache, ThumbnailCache, load_thumbnail};
use crate::constants::MAX_THUMBNAILS_PER_FRAME;

/// System that loads thumbnails and registers them with egui.
/// Runs in Update before the egui pass to avoid timing issues.
pub fn load_and_register_thumbnails(
    library: Res<AssetLibrary>,
    library_cache: Res<LibraryCache>,
    mut thumbnail_cache: ResMut<ThumbnailCache>,
    mut images: ResMut<Assets<Image>>,
    mut egui_textures: ResMut<EguiUserTextures>,
) {
    // Load a limited number of new thumbnails per frame to avoid stuttering
    let assets_to_load: Vec<&LibraryAsset> = library
        .assets
        .iter()
        .filter(|a| {
            !thumbnail_cache.thumbnails.contains_key(&a.full_path)
                && !thumbnail_cache.has_failed(&a.full_path)
        })
        .take(MAX_THUMBNAILS_PER_FRAME)
        .collect();

    // Cached thumbnails avoid decoding (and, for online-only files, downloading) the image
    for asset in assets_to_load {
        let path = asset.full_path.clone();
        if let Some(thumb_image) = load_thumbnail(asset, library_cache.dir.as_deref()) {
            let handle = images.add(thumb_image);
            thumbnail_cache.thumbnails.insert(path, handle);
        } else {
            thumbnail_cache.failed.insert(path, ());
        }
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
