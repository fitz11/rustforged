use bevy::prelude::*;
use std::path::PathBuf;

use super::{
    Layer, MapData, PlacedItem, SavedAnnotations, SavedLine, SavedMap, SavedPath, SavedPlacedItem,
    SavedTextBox,
};
use crate::assets::AssetLibrary;
use crate::editor::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};

#[derive(Message)]
pub struct SaveMapRequest {
    pub path: PathBuf,
}

#[derive(Message)]
pub struct LoadMapRequest {
    pub path: PathBuf,
}

#[derive(Message)]
pub struct NewMapRequest;

#[derive(Resource, Default)]
pub struct MapLoadError {
    pub message: Option<String>,
}

pub fn save_map_system(
    mut events: MessageReader<SaveMapRequest>,
    map_data: Res<MapData>,
    placed_items: Query<(&PlacedItem, &Transform)>,
    paths: Query<&DrawnPath>,
    lines: Query<&DrawnLine>,
    texts: Query<(&Transform, &TextAnnotation)>,
) {
    for event in events.read() {
        let items: Vec<SavedPlacedItem> = placed_items
            .iter()
            .map(|(item, transform)| SavedPlacedItem::from_entity(item, transform))
            .collect();

        // Collect annotations
        let saved_paths: Vec<SavedPath> = paths
            .iter()
            .map(|p| SavedPath {
                points: p.points.clone(),
                color: color_to_array(p.color),
                stroke_width: p.stroke_width,
            })
            .collect();

        let saved_lines: Vec<SavedLine> = lines
            .iter()
            .map(|l| SavedLine {
                start: l.start,
                end: l.end,
                color: color_to_array(l.color),
                stroke_width: l.stroke_width,
            })
            .collect();

        let saved_texts: Vec<SavedTextBox> = texts
            .iter()
            .map(|(transform, t)| SavedTextBox {
                position: transform.translation.truncate(),
                content: t.content.clone(),
                font_size: t.font_size,
                color: color_to_array(t.color),
            })
            .collect();

        let saved_map = SavedMap {
            map_data: map_data.clone(),
            placed_items: items,
            annotations: SavedAnnotations {
                paths: saved_paths,
                lines: saved_lines,
                text_boxes: saved_texts,
            },
        };

        match serde_json::to_string_pretty(&saved_map) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&event.path, json) {
                    error!("Failed to save map: {}", e);
                } else {
                    info!("Map saved to {:?}", event.path);
                }
            }
            Err(e) => {
                error!("Failed to serialize map: {}", e);
            }
        }
    }
}

fn color_to_array(color: Color) -> [f32; 4] {
    let srgba = color.to_srgba();
    [srgba.red, srgba.green, srgba.blue, srgba.alpha]
}

fn array_to_color(arr: [f32; 4]) -> Color {
    Color::srgba(arr[0], arr[1], arr[2], arr[3])
}

#[allow(clippy::too_many_arguments)]
pub fn load_map_system(
    mut commands: Commands,
    mut events: MessageReader<LoadMapRequest>,
    mut map_data: ResMut<MapData>,
    mut load_error: ResMut<MapLoadError>,
    asset_library: Res<AssetLibrary>,
    asset_server: Res<AssetServer>,
    existing_items: Query<Entity, With<PlacedItem>>,
    existing_annotations: Query<Entity, With<AnnotationMarker>>,
) {
    for event in events.read() {
        load_error.message = None;

        let json = match std::fs::read_to_string(&event.path) {
            Ok(content) => content,
            Err(e) => {
                load_error.message = Some(format!("Failed to read file: {}", e));
                error!("{}", load_error.message.as_ref().unwrap());
                continue;
            }
        };

        let saved_map: SavedMap = match serde_json::from_str(&json) {
            Ok(map) => map,
            Err(e) => {
                load_error.message = Some(format!("Failed to parse map file: {}", e));
                error!("{}", load_error.message.as_ref().unwrap());
                continue;
            }
        };

        // Validate all assets exist
        let mut missing_assets = Vec::new();
        for item in &saved_map.placed_items {
            let asset_exists = asset_library
                .assets
                .iter()
                .any(|a| a.relative_path == item.asset_path);
            if !asset_exists {
                missing_assets.push(item.asset_path.clone());
            }
        }

        if !missing_assets.is_empty() {
            load_error.message = Some(format!(
                "Cannot load map: missing assets:\n{}",
                missing_assets.join("\n")
            ));
            error!("{}", load_error.message.as_ref().unwrap());
            continue;
        }

        // Clear existing items
        for entity in existing_items.iter() {
            commands.entity(entity).despawn();
        }

        // Clear existing annotations
        for entity in existing_annotations.iter() {
            commands.entity(entity).despawn();
        }

        // Load map data
        *map_data = saved_map.map_data;

        // Spawn placed items
        for item in saved_map.placed_items {
            let texture: Handle<Image> = asset_server.load(&item.asset_path);
            let z = item.layer.z_base() + item.z_index as f32;

            commands.spawn((
                Sprite::from_image(texture),
                Transform {
                    translation: item.position.extend(z),
                    rotation: Quat::from_rotation_z(item.rotation),
                    scale: item.scale.extend(1.0),
                },
                PlacedItem {
                    asset_path: item.asset_path,
                    layer: item.layer,
                    z_index: item.z_index,
                },
            ));
        }

        // Spawn annotations
        let z = Layer::Annotation.z_base();

        for path in saved_map.annotations.paths {
            commands.spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                DrawnPath {
                    points: path.points,
                    color: array_to_color(path.color),
                    stroke_width: path.stroke_width,
                },
                AnnotationMarker,
            ));
        }

        for line in saved_map.annotations.lines {
            commands.spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                DrawnLine {
                    start: line.start,
                    end: line.end,
                    color: array_to_color(line.color),
                    stroke_width: line.stroke_width,
                },
                AnnotationMarker,
            ));
        }

        for text in saved_map.annotations.text_boxes {
            commands.spawn((
                Transform::from_translation(text.position.extend(z)),
                TextAnnotation {
                    content: text.content,
                    font_size: text.font_size,
                    color: array_to_color(text.color),
                },
                AnnotationMarker,
            ));
        }

        info!("Map loaded from {:?}", event.path);
    }
}

pub fn new_map_system(
    mut commands: Commands,
    mut events: MessageReader<NewMapRequest>,
    mut map_data: ResMut<MapData>,
    existing_items: Query<Entity, With<PlacedItem>>,
    existing_annotations: Query<Entity, With<AnnotationMarker>>,
) {
    for _ in events.read() {
        // Clear existing items
        for entity in existing_items.iter() {
            commands.entity(entity).despawn();
        }

        // Clear existing annotations
        for entity in existing_annotations.iter() {
            commands.entity(entity).despawn();
        }

        // Reset map data to default
        *map_data = MapData::default();

        info!("Created new map");
    }
}

pub fn ensure_maps_directory() {
    let maps_dir = PathBuf::from("assets/maps");
    if !maps_dir.exists()
        && let Err(e) = std::fs::create_dir_all(&maps_dir)
    {
        warn!("Failed to create maps directory: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // color_to_array tests
    #[test]
    fn test_color_to_array_red() {
        let color = Color::srgba(1.0, 0.0, 0.0, 1.0);
        let arr = color_to_array(color);
        assert_eq!(arr, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_color_to_array_green() {
        let color = Color::srgba(0.0, 1.0, 0.0, 1.0);
        let arr = color_to_array(color);
        assert_eq!(arr, [0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn test_color_to_array_blue() {
        let color = Color::srgba(0.0, 0.0, 1.0, 1.0);
        let arr = color_to_array(color);
        assert_eq!(arr, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_color_to_array_with_alpha() {
        let color = Color::srgba(0.5, 0.5, 0.5, 0.5);
        let arr = color_to_array(color);
        assert!((arr[0] - 0.5).abs() < 0.001);
        assert!((arr[1] - 0.5).abs() < 0.001);
        assert!((arr[2] - 0.5).abs() < 0.001);
        assert!((arr[3] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_color_to_array_white() {
        let color = Color::srgba(1.0, 1.0, 1.0, 1.0);
        let arr = color_to_array(color);
        assert_eq!(arr, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_color_to_array_black() {
        let color = Color::srgba(0.0, 0.0, 0.0, 1.0);
        let arr = color_to_array(color);
        assert_eq!(arr, [0.0, 0.0, 0.0, 1.0]);
    }

    // array_to_color tests
    #[test]
    fn test_array_to_color_red() {
        let arr = [1.0, 0.0, 0.0, 1.0];
        let color = array_to_color(arr);
        let srgba = color.to_srgba();
        assert_eq!(srgba.red, 1.0);
        assert_eq!(srgba.green, 0.0);
        assert_eq!(srgba.blue, 0.0);
        assert_eq!(srgba.alpha, 1.0);
    }

    #[test]
    fn test_array_to_color_partial_alpha() {
        let arr = [0.25, 0.5, 0.75, 0.5];
        let color = array_to_color(arr);
        let srgba = color.to_srgba();
        assert!((srgba.red - 0.25).abs() < 0.001);
        assert!((srgba.green - 0.5).abs() < 0.001);
        assert!((srgba.blue - 0.75).abs() < 0.001);
        assert!((srgba.alpha - 0.5).abs() < 0.001);
    }

    // Round-trip tests
    #[test]
    fn test_color_roundtrip() {
        let original = Color::srgba(0.2, 0.4, 0.6, 0.8);
        let arr = color_to_array(original);
        let recovered = array_to_color(arr);
        let original_srgba = original.to_srgba();
        let recovered_srgba = recovered.to_srgba();

        assert!((original_srgba.red - recovered_srgba.red).abs() < 0.001);
        assert!((original_srgba.green - recovered_srgba.green).abs() < 0.001);
        assert!((original_srgba.blue - recovered_srgba.blue).abs() < 0.001);
        assert!((original_srgba.alpha - recovered_srgba.alpha).abs() < 0.001);
    }

    #[test]
    fn test_color_roundtrip_multiple() {
        let colors = [
            Color::srgba(1.0, 0.0, 0.0, 1.0),
            Color::srgba(0.0, 1.0, 0.0, 1.0),
            Color::srgba(0.0, 0.0, 1.0, 1.0),
            Color::srgba(1.0, 1.0, 1.0, 1.0),
            Color::srgba(0.0, 0.0, 0.0, 1.0),
            Color::srgba(0.5, 0.5, 0.5, 0.5),
            Color::srgba(0.1, 0.2, 0.3, 0.4),
        ];

        for original in colors {
            let arr = color_to_array(original);
            let recovered = array_to_color(arr);
            let original_srgba = original.to_srgba();
            let recovered_srgba = recovered.to_srgba();

            assert!(
                (original_srgba.red - recovered_srgba.red).abs() < 0.001,
                "Red mismatch for {:?}",
                original
            );
            assert!(
                (original_srgba.green - recovered_srgba.green).abs() < 0.001,
                "Green mismatch for {:?}",
                original
            );
            assert!(
                (original_srgba.blue - recovered_srgba.blue).abs() < 0.001,
                "Blue mismatch for {:?}",
                original
            );
            assert!(
                (original_srgba.alpha - recovered_srgba.alpha).abs() < 0.001,
                "Alpha mismatch for {:?}",
                original
            );
        }
    }

    // MapLoadError tests
    #[test]
    fn test_map_load_error_default() {
        let error = MapLoadError::default();
        assert!(error.message.is_none());
    }
}
