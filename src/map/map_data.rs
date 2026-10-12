use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::assets::AssetFingerprint;

use super::{Layer, PlacedItem, SavedFogOfWar};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct MapData {
    pub name: String,
    pub grid_size: f32,
    /// Grid lines in the editor
    pub grid_visible: bool,
    /// Grid lines in the player view. Separate from the editor's, because battlemaps
    /// often have a grid drawn in already
    #[serde(default = "default_true")]
    pub player_grid_visible: bool,
    pub layers: Vec<LayerData>,
}

fn default_true() -> bool {
    true
}

impl Default for MapData {
    fn default() -> Self {
        Self {
            name: "Untitled Map".to_string(),
            grid_size: DEFAULT_GRID_SIZE,
            grid_visible: true,
            player_grid_visible: true,
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

/// Grid sizes the app accepts, in world units per cell; outside this a grid would be
/// unusable, and a size of zero hangs the grid and fog drawing
pub const GRID_SIZE_RANGE: std::ops::RangeInclusive<f32> = 10.0..=1000.0;
/// Grid size of a new map
pub const DEFAULT_GRID_SIZE: f32 = 70.0;

impl MapData {
    /// Repairs settings a hand-edited or damaged map file can get wrong: a grid size outside
    /// [`GRID_SIZE_RANGE`], and layers missing from or repeated in the layer list (a missing
    /// layer had no row in the Layers panel, so it couldn't be shown or hidden).
    pub fn normalize(&mut self) {
        if !self.grid_size.is_finite() || !GRID_SIZE_RANGE.contains(&self.grid_size) {
            warn!(
                "Map grid size {} is out of range, using {}",
                self.grid_size, DEFAULT_GRID_SIZE
            );
            self.grid_size = DEFAULT_GRID_SIZE;
        }
        let defaults = MapData::default().layers;
        self.layers = defaults
            .into_iter()
            .map(|default| self.layer(default.layer_type).cloned().unwrap_or(default))
            .collect();
    }

    /// Settings for `layer`, if the map has them.
    pub fn layer(&self, layer: Layer) -> Option<&LayerData> {
        self.layers.iter().find(|ld| ld.layer_type == layer)
    }

    /// Settings for `layer`, mutably, if the map has them.
    pub fn layer_mut(&mut self, layer: Layer) -> Option<&mut LayerData> {
        self.layers.iter_mut().find(|ld| ld.layer_type == layer)
    }

    /// Whether `layer` is shown. A layer the map has no settings for is shown.
    pub fn is_layer_visible(&self, layer: Layer) -> bool {
        self.layer(layer).is_none_or(|ld| ld.visible)
    }

    /// Whether `layer` is locked against edits.
    pub fn is_layer_locked(&self, layer: Layer) -> bool {
        self.layer(layer).is_some_and(|ld| ld.locked)
    }

    /// Whether items on `layer` can be clicked, selected and edited: shown and unlocked.
    pub fn is_layer_editable(&self, layer: Layer) -> bool {
        self.is_layer_visible(layer) && !self.is_layer_locked(layer)
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
    /// A fixed order for items that are otherwise level: by asset, then placement
    pub fn canonical_cmp(&self, other: &Self) -> Ordering {
        fn vec2(a: Vec2, b: Vec2) -> Ordering {
            a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y))
        }
        self.asset_path
            .cmp(&other.asset_path)
            .then_with(|| vec2(self.position, other.position))
            .then(self.rotation.total_cmp(&other.rotation))
            .then_with(|| vec2(self.scale, other.scale))
    }

    /// The item's transform; its depth is set when it's spawned.
    pub fn transform(&self) -> Transform {
        Transform {
            translation: self.position.extend(0.0),
            rotation: Quat::from_rotation_z(self.rotation),
            scale: self.scale.extend(1.0),
        }
    }

    /// The item's component, without its position.
    pub fn placed_item(&self) -> PlacedItem {
        PlacedItem {
            asset_path: self.asset_path.clone(),
            layer: self.layer,
            z_index: self.z_index,
        }
    }

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

/// Map file format this version writes. Files without a version predate versioning.
pub const MAP_FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMap {
    #[serde(default)]
    pub format_version: u32,
    /// Stable identifier, kept across saves and renames; a copy saved under a new name gets
    /// a new one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_id: Option<String>,
    /// Host name of the machine that last saved the map
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_by: Option<String>,
    /// Content fingerprint of each referenced asset, by library-relative path, as of the
    /// last save. Lets a reference be found again after its file was renamed or moved.
    #[serde(default)]
    pub asset_fingerprints: BTreeMap<String, AssetFingerprint>,
    pub map_data: MapData,
    pub placed_items: Vec<SavedPlacedItem>,
    #[serde(default)]
    pub annotations: SavedAnnotations,
    #[serde(default)]
    pub fog_of_war: SavedFogOfWar,
}

impl SavedMap {
    /// Repairs what a hand-edited or damaged file can get wrong (see [`MapData::normalize`]),
    /// and drops items and annotations with non-finite coordinates, which can't be drawn.
    pub fn normalize(&mut self) {
        self.map_data.normalize();
        let finite = |v: Vec2| v.is_finite();
        self.placed_items.retain(|item| {
            finite(item.position) && finite(item.scale) && item.rotation.is_finite()
        });
        self.annotations
            .paths
            .retain(|path| path.points.iter().copied().all(finite));
        self.annotations
            .lines
            .retain(|line| finite(line.start) && finite(line.end));
        self.annotations
            .text_boxes
            .retain(|text| finite(text.position) && text.font_size.is_finite());
    }

    /// Point placed items at a different asset path (after a rename, move or relink).
    /// Returns true if any item changed.
    pub fn retarget_asset(&mut self, old_path: &str, new_path: &str) -> bool {
        let mut changed = false;
        for item in self
            .placed_items
            .iter_mut()
            .filter(|i| i.asset_path == old_path)
        {
            item.asset_path = new_path.to_string();
            changed = true;
        }
        changed
    }

    /// Puts items and annotations in a canonical order, so saving an unchanged map produces
    /// identical bytes (entity iteration order varies from run to run).
    pub fn sort_for_save(&mut self) {
        fn vec2(a: Vec2, b: Vec2) -> Ordering {
            a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y))
        }
        fn color(a: &[f32; 4], b: &[f32; 4]) -> Ordering {
            a.iter()
                .zip(b)
                .map(|(x, y)| x.total_cmp(y))
                .find(|o| o.is_ne())
                .unwrap_or(Ordering::Equal)
        }

        // By layer and z-index; within those the list is already in stacking order (see
        // capture_current_map_state), which the sort keeps
        self.placed_items.sort_by(|a, b| {
            a.layer
                .z_base()
                .total_cmp(&b.layer.z_base())
                .then(a.z_index.cmp(&b.z_index))
        });
        self.annotations.paths.sort_by(|a, b| {
            a.points
                .iter()
                .zip(&b.points)
                .map(|(p, q)| vec2(*p, *q))
                .find(|o| o.is_ne())
                .unwrap_or(Ordering::Equal)
                .then(a.points.len().cmp(&b.points.len()))
                .then_with(|| color(&a.color, &b.color))
                .then(a.stroke_width.total_cmp(&b.stroke_width))
        });
        self.annotations.lines.sort_by(|a, b| {
            vec2(a.start, b.start)
                .then_with(|| vec2(a.end, b.end))
                .then_with(|| color(&a.color, &b.color))
                .then(a.stroke_width.total_cmp(&b.stroke_width))
        });
        self.annotations.text_boxes.sort_by(|a, b| {
            vec2(a.position, b.position)
                .then_with(|| a.content.cmp(&b.content))
                .then(a.font_size.total_cmp(&b.font_size))
                .then_with(|| color(&a.color, &b.color))
        });
    }

    /// Unique library-relative paths of the assets this map places.
    pub fn used_assets(&self) -> BTreeSet<String> {
        self.placed_items
            .iter()
            .map(|i| i.asset_path.clone())
            .collect()
    }
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
                assert!(
                    !layer_data.visible,
                    "FogOfWar layer should be disabled by default"
                );
            } else {
                assert!(
                    layer_data.visible,
                    "Layer {:?} should be visible by default",
                    layer_data.layer_type
                );
            }
        }
    }

    #[test]
    fn test_map_data_default_layers_all_unlocked() {
        let map_data = MapData::default();
        for layer_data in &map_data.layers {
            assert!(
                !layer_data.locked,
                "Layer {:?} should be unlocked by default",
                layer_data.layer_type
            );
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

    // SavedMap tests
    #[test]
    fn test_saved_map_serialization() {
        let saved_map = SavedMap {
            format_version: MAP_FORMAT_VERSION,
            map_id: None,
            saved_by: None,
            asset_fingerprints: BTreeMap::new(),
            map_data: MapData::default(),
            placed_items: vec![],
            annotations: SavedAnnotations::default(),
            fog_of_war: SavedFogOfWar::default(),
        };

        let json = serde_json::to_string(&saved_map).unwrap();
        let deserialized: SavedMap = serde_json::from_str(&json).unwrap();

        assert_eq!(saved_map.map_data.name, deserialized.map_data.name);
        assert_eq!(
            saved_map.placed_items.len(),
            deserialized.placed_items.len()
        );
        assert!(deserialized.asset_fingerprints.is_empty());
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
        let saved_map = SavedMap {
            format_version: MAP_FORMAT_VERSION,
            map_id: None,
            saved_by: None,
            asset_fingerprints: BTreeMap::new(),
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
        assert_eq!(deserialized.used_assets().len(), 2);
    }

    #[test]
    fn test_saved_map_annotations_default_on_deserialize() {
        // Test that annotations and asset_fingerprints default to empty when not present in JSON
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
        assert!(deserialized.asset_fingerprints.is_empty());
    }

    #[test]
    fn test_normalize_repairs_grid_size_and_layers() {
        let mut map = MapData {
            grid_size: 0.0,
            layers: vec![
                LayerData {
                    layer_type: Layer::Token,
                    visible: false,
                    locked: true,
                },
                LayerData {
                    layer_type: Layer::Token,
                    visible: true,
                    locked: false,
                },
            ],
            ..MapData::default()
        };
        map.normalize();

        assert_eq!(map.grid_size, DEFAULT_GRID_SIZE);
        assert_eq!(map.layers.len(), Layer::all().len());
        // The first entry for a layer wins; missing layers get their defaults
        assert!(!map.is_layer_visible(Layer::Token));
        assert!(map.is_layer_locked(Layer::Token));
        assert!(!map.is_layer_visible(Layer::FogOfWar));
        assert!(map.is_layer_visible(Layer::Background));

        map.grid_size = f32::NAN;
        map.normalize();
        assert_eq!(map.grid_size, DEFAULT_GRID_SIZE);

        map.grid_size = 140.0;
        map.normalize();
        assert_eq!(map.grid_size, 140.0);
    }
}
