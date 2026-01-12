use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::{Layer, PlacedItem, SavedFogOfWar};

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
                    // FogOfWar layer is disabled by default (fog data exists but isn't rendered)
                    visible: *layer != Layer::FogOfWar,
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

/// Manifest of all assets used in a map.
///
/// Stored at the top of map files for quick validation without
/// scanning all placed items. Used to verify all required assets
/// exist before loading a map.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetManifest {
    /// Unique list of asset paths used in the map
    pub assets: Vec<String>,
}

impl AssetManifest {
    /// Build a manifest from a collection of placed items
    pub fn from_items<'a>(items: impl Iterator<Item = &'a SavedPlacedItem>) -> Self {
        let unique: HashSet<_> = items.map(|item| item.asset_path.clone()).collect();
        let mut assets: Vec<_> = unique.into_iter().collect();
        assets.sort(); // Deterministic ordering for diffs
        Self { assets }
    }

    /// Check if the manifest is empty (no assets used)
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// Get the number of unique assets in the manifest
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.assets.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMap {
    /// Asset manifest for quick validation (stored first in JSON)
    #[serde(default)]
    pub asset_manifest: AssetManifest,
    pub map_data: MapData,
    pub placed_items: Vec<SavedPlacedItem>,
    #[serde(default)]
    pub annotations: SavedAnnotations,
    #[serde(default)]
    pub fog_of_war: SavedFogOfWar,
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

#[cfg(test)]
mod tests {
    use super::*;

    // MapData tests
    #[test]
    fn test_map_data_default_name() {
        let map_data = MapData::default();
        assert_eq!(map_data.name, "Untitled Map");
    }

    #[test]
    fn test_map_data_default_grid_size() {
        let map_data = MapData::default();
        assert_eq!(map_data.grid_size, 70.0);
    }

    #[test]
    fn test_map_data_default_grid_visible() {
        let map_data = MapData::default();
        assert!(map_data.grid_visible);
    }

    #[test]
    fn test_map_data_default_layers_count() {
        let map_data = MapData::default();
        assert_eq!(map_data.layers.len(), Layer::all().len());
    }

    #[test]
    fn test_map_data_default_layers_visibility() {
        let map_data = MapData::default();
        for layer_data in &map_data.layers {
            if layer_data.layer_type == Layer::FogOfWar {
                // FogOfWar layer is disabled by default
                assert!(!layer_data.visible, "FogOfWar layer should be disabled by default");
            } else {
                assert!(layer_data.visible, "Layer {:?} should be visible by default", layer_data.layer_type);
            }
        }
    }

    #[test]
    fn test_map_data_default_layers_all_unlocked() {
        let map_data = MapData::default();
        for layer_data in &map_data.layers {
            assert!(!layer_data.locked, "Layer {:?} should be unlocked by default", layer_data.layer_type);
        }
    }

    #[test]
    fn test_map_data_serialization_roundtrip() {
        let map_data = MapData::default();
        let json = serde_json::to_string(&map_data).unwrap();
        let deserialized: MapData = serde_json::from_str(&json).unwrap();

        assert_eq!(map_data.name, deserialized.name);
        assert_eq!(map_data.grid_size, deserialized.grid_size);
        assert_eq!(map_data.grid_visible, deserialized.grid_visible);
        assert_eq!(map_data.layers.len(), deserialized.layers.len());
    }

    // LayerData tests
    #[test]
    fn test_layer_data_serialization() {
        let layer_data = LayerData {
            layer_type: Layer::Token,
            visible: false,
            locked: true,
        };

        let json = serde_json::to_string(&layer_data).unwrap();
        let deserialized: LayerData = serde_json::from_str(&json).unwrap();

        assert_eq!(layer_data.layer_type, deserialized.layer_type);
        assert_eq!(layer_data.visible, deserialized.visible);
        assert_eq!(layer_data.locked, deserialized.locked);
    }

    // SavedPlacedItem tests
    #[test]
    fn test_saved_placed_item_from_entity() {
        let placed_item = PlacedItem {
            asset_path: "library/tokens/hero.png".to_string(),
            layer: Layer::Token,
            z_index: 5,
        };

        let transform = Transform {
            translation: Vec3::new(100.0, 200.0, 305.0),
            rotation: Quat::from_rotation_z(std::f32::consts::PI / 4.0),
            scale: Vec3::new(2.0, 2.0, 1.0),
        };

        let saved = SavedPlacedItem::from_entity(&placed_item, &transform);

        assert_eq!(saved.asset_path, "library/tokens/hero.png");
        assert_eq!(saved.position, Vec2::new(100.0, 200.0));
        assert_eq!(saved.scale, Vec2::new(2.0, 2.0));
        assert_eq!(saved.layer, Layer::Token);
        assert_eq!(saved.z_index, 5);
        // Rotation should be approximately PI/4
        assert!((saved.rotation - std::f32::consts::PI / 4.0).abs() < 0.001);
    }

    #[test]
    fn test_saved_placed_item_serialization() {
        let saved = SavedPlacedItem {
            asset_path: "test.png".to_string(),
            position: Vec2::new(10.0, 20.0),
            rotation: 0.5,
            scale: Vec2::new(1.0, 1.0),
            layer: Layer::Doodad,
            z_index: 3,
        };

        let json = serde_json::to_string(&saved).unwrap();
        let deserialized: SavedPlacedItem = serde_json::from_str(&json).unwrap();

        assert_eq!(saved.asset_path, deserialized.asset_path);
        assert_eq!(saved.position, deserialized.position);
        assert_eq!(saved.rotation, deserialized.rotation);
        assert_eq!(saved.scale, deserialized.scale);
        assert_eq!(saved.layer, deserialized.layer);
        assert_eq!(saved.z_index, deserialized.z_index);
    }

    // SavedAnnotations tests
    #[test]
    fn test_saved_annotations_default() {
        let annotations = SavedAnnotations::default();
        assert!(annotations.paths.is_empty());
        assert!(annotations.lines.is_empty());
        assert!(annotations.text_boxes.is_empty());
    }

    #[test]
    fn test_saved_path_serialization() {
        let path = SavedPath {
            points: vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0)],
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_width: 3.0,
        };

        let json = serde_json::to_string(&path).unwrap();
        let deserialized: SavedPath = serde_json::from_str(&json).unwrap();

        assert_eq!(path.points, deserialized.points);
        assert_eq!(path.color, deserialized.color);
        assert_eq!(path.stroke_width, deserialized.stroke_width);
    }

    #[test]
    fn test_saved_line_serialization() {
        let line = SavedLine {
            start: Vec2::new(0.0, 0.0),
            end: Vec2::new(100.0, 100.0),
            color: [0.0, 1.0, 0.0, 1.0],
            stroke_width: 2.0,
        };

        let json = serde_json::to_string(&line).unwrap();
        let deserialized: SavedLine = serde_json::from_str(&json).unwrap();

        assert_eq!(line.start, deserialized.start);
        assert_eq!(line.end, deserialized.end);
        assert_eq!(line.color, deserialized.color);
        assert_eq!(line.stroke_width, deserialized.stroke_width);
    }

    #[test]
    fn test_saved_text_box_serialization() {
        let text_box = SavedTextBox {
            position: Vec2::new(50.0, 75.0),
            content: "Hello World".to_string(),
            font_size: 16.0,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let json = serde_json::to_string(&text_box).unwrap();
        let deserialized: SavedTextBox = serde_json::from_str(&json).unwrap();

        assert_eq!(text_box.position, deserialized.position);
        assert_eq!(text_box.content, deserialized.content);
        assert_eq!(text_box.font_size, deserialized.font_size);
        assert_eq!(text_box.color, deserialized.color);
    }

    // AssetManifest tests
    #[test]
    fn test_asset_manifest_default() {
        let manifest = AssetManifest::default();
        assert!(manifest.is_empty());
        assert_eq!(manifest.len(), 0);
    }

    #[test]
    fn test_asset_manifest_from_items() {
        let items = [
            SavedPlacedItem {
                asset_path: "tokens/hero.png".to_string(),
                position: Vec2::ZERO,
                rotation: 0.0,
                scale: Vec2::ONE,
                layer: Layer::Token,
                z_index: 0,
            },
            SavedPlacedItem {
                asset_path: "terrain/grass.png".to_string(),
                position: Vec2::ZERO,
                rotation: 0.0,
                scale: Vec2::ONE,
                layer: Layer::Terrain,
                z_index: 0,
            },
            SavedPlacedItem {
                asset_path: "tokens/hero.png".to_string(), // Duplicate
                position: Vec2::ONE,
                rotation: 0.0,
                scale: Vec2::ONE,
                layer: Layer::Token,
                z_index: 1,
            },
        ];

        let manifest = AssetManifest::from_items(items.iter());

        // Should have only 2 unique assets
        assert_eq!(manifest.len(), 2);
        assert!(manifest.assets.contains(&"tokens/hero.png".to_string()));
        assert!(manifest.assets.contains(&"terrain/grass.png".to_string()));
    }

    #[test]
    fn test_asset_manifest_sorted() {
        let items = [
            SavedPlacedItem {
                asset_path: "z_last.png".to_string(),
                position: Vec2::ZERO,
                rotation: 0.0,
                scale: Vec2::ONE,
                layer: Layer::Token,
                z_index: 0,
            },
            SavedPlacedItem {
                asset_path: "a_first.png".to_string(),
                position: Vec2::ZERO,
                rotation: 0.0,
                scale: Vec2::ONE,
                layer: Layer::Token,
                z_index: 0,
            },
        ];

        let manifest = AssetManifest::from_items(items.iter());

        // Assets should be sorted alphabetically
        assert_eq!(manifest.assets[0], "a_first.png");
        assert_eq!(manifest.assets[1], "z_last.png");
    }

    #[test]
    fn test_asset_manifest_serialization() {
        let manifest = AssetManifest {
            assets: vec!["a.png".to_string(), "b.png".to_string()],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: AssetManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(manifest.assets, deserialized.assets);
    }

    // SavedMap tests
    #[test]
    fn test_saved_map_serialization() {
        let saved_map = SavedMap {
            asset_manifest: AssetManifest::default(),
            map_data: MapData::default(),
            placed_items: vec![],
            annotations: SavedAnnotations::default(),
            fog_of_war: SavedFogOfWar::default(),
        };

        let json = serde_json::to_string(&saved_map).unwrap();
        let deserialized: SavedMap = serde_json::from_str(&json).unwrap();

        assert_eq!(saved_map.map_data.name, deserialized.map_data.name);
        assert_eq!(saved_map.placed_items.len(), deserialized.placed_items.len());
        assert!(deserialized.asset_manifest.is_empty());
    }

    #[test]
    fn test_saved_map_with_items() {
        let items = vec![
            SavedPlacedItem {
                asset_path: "item1.png".to_string(),
                position: Vec2::new(0.0, 0.0),
                rotation: 0.0,
                scale: Vec2::ONE,
                layer: Layer::Token,
                z_index: 0,
            },
            SavedPlacedItem {
                asset_path: "item2.png".to_string(),
                position: Vec2::new(100.0, 100.0),
                rotation: 1.0,
                scale: Vec2::splat(2.0),
                layer: Layer::Doodad,
                z_index: 1,
            },
        ];
        let manifest = AssetManifest::from_items(items.iter());

        let saved_map = SavedMap {
            asset_manifest: manifest,
            map_data: MapData {
                name: "Test Map".to_string(),
                ..MapData::default()
            },
            placed_items: items,
            annotations: SavedAnnotations::default(),
            fog_of_war: SavedFogOfWar::default(),
        };

        let json = serde_json::to_string(&saved_map).unwrap();
        let deserialized: SavedMap = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.map_data.name, "Test Map");
        assert_eq!(deserialized.placed_items.len(), 2);
        assert_eq!(deserialized.placed_items[0].asset_path, "item1.png");
        assert_eq!(deserialized.placed_items[1].asset_path, "item2.png");
        assert_eq!(deserialized.asset_manifest.len(), 2);
    }

    #[test]
    fn test_saved_map_annotations_default_on_deserialize() {
        // Test that annotations and asset_manifest default to empty when not present in JSON
        // (simulating loading an old save file without these fields)
        let json = r#"{
            "map_data": {
                "name": "Old Map",
                "grid_size": 70.0,
                "grid_visible": true,
                "layers": []
            },
            "placed_items": []
        }"#;

        let deserialized: SavedMap = serde_json::from_str(json).unwrap();
        assert!(deserialized.annotations.paths.is_empty());
        assert!(deserialized.annotations.lines.is_empty());
        assert!(deserialized.annotations.text_boxes.is_empty());
        assert!(deserialized.asset_manifest.is_empty());
    }
}
