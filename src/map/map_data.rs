use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Layer, PlacedItem};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct MapData {
    pub name: String,
    pub grid_size: f32,
    pub grid_visible: bool,
    pub layers: Vec<LayerData>,
}

impl Default for MapData {
    fn default() -> Self {
        Self {
            name: "Untitled Map".to_string(),
            grid_size: 70.0,
            grid_visible: true,
            layers: Layer::all()
                .iter()
                .map(|layer| LayerData {
                    layer_type: *layer,
                    visible: true,
                    locked: false,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerData {
    pub layer_type: Layer,
    pub visible: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPlacedItem {
    pub asset_path: String,
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub layer: Layer,
    pub z_index: i32,
}

impl SavedPlacedItem {
    pub fn from_entity(item: &PlacedItem, transform: &Transform) -> Self {
        Self {
            asset_path: item.asset_path.clone(),
            position: transform.translation.truncate(),
            rotation: transform.rotation.to_euler(EulerRot::ZYX).0,
            scale: transform.scale.truncate(),
            layer: item.layer,
            z_index: item.z_index,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMap {
    pub map_data: MapData,
    pub placed_items: Vec<SavedPlacedItem>,
    #[serde(default)]
    pub annotations: SavedAnnotations,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedAnnotations {
    pub paths: Vec<SavedPath>,
    pub lines: Vec<SavedLine>,
    pub text_boxes: Vec<SavedTextBox>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPath {
    pub points: Vec<Vec2>,
    pub color: [f32; 4],
    pub stroke_width: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedLine {
    pub start: Vec2,
    pub end: Vec2,
    pub color: [f32; 4],
    pub stroke_width: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedTextBox {
    pub position: Vec2,
    pub content: String,
    pub font_size: f32,
    pub color: [f32; 4],
}
