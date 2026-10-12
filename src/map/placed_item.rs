use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::assets::library_asset_path;

use super::Layer;

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct PlacedItem {
    pub asset_path: String,
    pub layer: Layer,
    pub z_index: i32,
}

impl PlacedItem {
    /// Components for a placed item entity: its sprite, `transform` moved to the item's
    /// depth, and render layers that keep GM-only items out of the player view. Every
    /// place that spawns an item uses this, so they can't disagree about what players see.
    pub fn bundle(self, asset_server: &AssetServer, mut transform: Transform) -> impl Bundle {
        let z_index = self.z_index.clamp(0, Layer::max_z_index());
        let item = PlacedItem { z_index, ..self };
        transform.translation.z = item.depth();
        let texture: Handle<Image> = asset_server.load(library_asset_path(&item.asset_path));
        let render_layers = item.layer.render_layers();
        (Sprite::from_image(texture), transform, item, render_layers)
    }

    /// The item's depth: its layer's base plus its z-index within the layer.
    pub fn depth(&self) -> f32 {
        self.layer.z_base() + self.z_index as f32
    }
}

#[derive(Component)]
pub struct Selected;

/// Marker component for items whose asset file is missing or failed to load.
/// The original_path stores the path that was requested but couldn't be found.
#[derive(Component)]
pub struct MissingAsset {
    /// The original asset path that was requested but couldn't be found.
    /// Used for displaying error information in the UI.
    #[allow(dead_code)]
    pub original_path: String,
}
