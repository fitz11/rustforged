//! Asset validation system for detecting and handling missing assets.
//!
//! Detects when placed items reference assets that fail to load and
//! applies placeholder textures to make them visible in the editor.

use bevy::asset::LoadState;
use bevy::prelude::*;

use crate::map::{MissingAsset, PlacedItem};

use super::placeholder::PlaceholderTexture;
use super::{AssetLibrary, library_asset_path};

/// System that checks for failed asset loads and applies placeholder textures.
///
/// Runs lazily - queries items without MissingAsset marker and checks if their
/// sprite texture failed to load. A `lib://` path that doesn't exist in the open
/// library fails with "not found", so this also covers deleted or not-yet-synced files.
pub fn detect_missing_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    placeholder: Res<PlaceholderTexture>,
    query: Query<(Entity, &PlacedItem, &Sprite), Without<MissingAsset>>,
) {
    for (entity, placed_item, sprite) in query.iter() {
        if let LoadState::Failed(_) = asset_server.load_state(&sprite.image) {
            warn!("Asset failed to load: {}", placed_item.asset_path);
            commands.entity(entity).insert((
                MissingAsset {
                    original_path: placed_item.asset_path.clone(),
                },
                Sprite {
                    image: placeholder.handle.clone(),
                    custom_size: Some(Vec2::splat(64.0)),
                    ..default()
                },
            ));
        }
    }
}

/// System that swaps placeholders back to real images once the library contains their
/// asset again, e.g. after a sync client finishes downloading it or another library opens.
pub fn restore_found_assets(
    mut commands: Commands,
    library: Res<AssetLibrary>,
    mut last_generation: Local<u64>,
    asset_server: Res<AssetServer>,
    query: Query<(Entity, &PlacedItem), With<MissingAsset>>,
) {
    if *last_generation == library.generation() {
        return;
    }
    *last_generation = library.generation();

    for (entity, placed_item) in query.iter() {
        if library.contains(&placed_item.asset_path) {
            info!("Asset found again: {}", placed_item.asset_path);
            commands
                .entity(entity)
                .remove::<MissingAsset>()
                .insert(Sprite::from_image(
                    asset_server.load(library_asset_path(&placed_item.asset_path)),
                ));
        }
    }
}

/// Draw visual indicators for missing assets.
///
/// Renders a red border around items with missing assets to make them
/// more visible in the editor.
pub fn draw_missing_asset_indicators(
    mut gizmos: Gizmos,
    query: Query<&Transform, (With<PlacedItem>, With<MissingAsset>)>,
) {
    let indicator_color = Color::srgba(1.0, 0.3, 0.3, 0.9);

    for transform in query.iter() {
        let pos = transform.translation.truncate();

        // Draw red border around the placeholder (slightly larger than 64x64)
        gizmos.rect_2d(
            Isometry2d::from_translation(pos),
            Vec2::splat(70.0),
            indicator_color,
        );
    }
}
