//! Asset validation system for detecting and handling missing assets.
//!
//! Detects when placed items reference assets that fail to load and
//! applies placeholder textures to make them visible in the editor.

use bevy::asset::LoadState;
use bevy::prelude::*;

use crate::map::{MissingAsset, PlacedItem};

use super::placeholder::PlaceholderTexture;

/// System that checks for failed asset loads and applies placeholder textures.
///
/// Runs lazily - queries items without MissingAsset marker and checks if their
/// sprite texture failed to load.
pub fn detect_missing_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    placeholder: Res<PlaceholderTexture>,
    query: Query<(Entity, &PlacedItem, &Sprite), Without<MissingAsset>>,
) {
    for (entity, placed_item, sprite) in query.iter() {
        // Check the load state of the sprite's image
        let load_state = asset_server.load_state(&sprite.image);

        match load_state {
            LoadState::Failed(_) => {
                // Asset failed to load - apply placeholder
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
            LoadState::NotLoaded => {
                // Asset not loaded yet - check if file exists on disk
                // This catches cases where the file was deleted after being referenced
                let full_path = if placed_item.asset_path.starts_with("library/") {
                    // Default library path
                    std::path::PathBuf::from("assets").join(&placed_item.asset_path)
                } else {
                    // External library - path is already absolute or relative
                    std::path::PathBuf::from(&placed_item.asset_path)
                };

                if !full_path.exists() {
                    warn!(
                        "Asset file not found: {} (checked: {:?})",
                        placed_item.asset_path, full_path
                    );
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
            LoadState::Loading | LoadState::Loaded => {
                // Still loading or successfully loaded - no action needed
            }
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
