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
