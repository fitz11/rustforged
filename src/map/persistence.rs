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
    if !maps_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&maps_dir) {
            warn!("Failed to create maps directory: {}", e);
        }
    }
}
