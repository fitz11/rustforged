use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::Layer;

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct PlacedItem {
    pub asset_path: String,
    pub layer: Layer,
    pub z_index: i32,
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
