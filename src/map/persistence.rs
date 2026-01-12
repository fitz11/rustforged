use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

use super::{
    Layer, MapData, PlacedItem, SavedAnnotations, SavedLine, SavedMap, SavedPath, SavedPlacedItem,
    SavedTextBox,
};
use crate::assets::AssetLibrary;
use crate::config::UpdateLastMapPath;
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

/// Message to request switching to a different open map
#[derive(Message)]
#[allow(dead_code)] // Reserved for future map switching feature
pub struct SwitchMapRequest {
    pub map_id: u64,
}

#[derive(Resource, Default)]
pub struct MapLoadError {
    pub message: Option<String>,
}

/// Resource tracking the currently loaded map file path
#[derive(Resource, Default)]
pub struct CurrentMapFile {
    pub path: Option<PathBuf>,
}

/// Resource tracking if the current map has unsaved changes
#[derive(Resource, Default)]
pub struct MapDirtyState {
    pub is_dirty: bool,
    /// Count of entities when map was last saved/loaded (for change detection)
    pub last_known_item_count: usize,
    pub last_known_annotation_count: usize,
}

/// Represents a map that's open in memory
#[derive(Clone)]
pub struct OpenMap {
    pub id: u64,
    pub name: String,
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    pub saved_state: Option<SavedMap>,
}

/// Resource tracking all open maps
#[derive(Resource)]
pub struct OpenMaps {
    pub maps: HashMap<u64, OpenMap>,
    pub active_map_id: Option<u64>,
    pub next_id: u64,
}

impl Default for OpenMaps {
    fn default() -> Self {
        // Start with one untitled map
        let mut maps = HashMap::new();
        maps.insert(
            0,
            OpenMap {
                id: 0,
                name: "Untitled Map".to_string(),
                path: None,
                is_dirty: false,
                saved_state: None,
            },
        );
        Self {
            maps,
            active_map_id: Some(0),
            next_id: 1,
        }
    }
}

impl OpenMaps {
    /// Get the currently active map
    #[allow(dead_code)]
    pub fn active_map(&self) -> Option<&OpenMap> {
        self.active_map_id.and_then(|id| self.maps.get(&id))
    }

    /// Get the currently active map mutably
    pub fn active_map_mut(&mut self) -> Option<&mut OpenMap> {
        self.active_map_id.and_then(|id| self.maps.get_mut(&id))
    }

    /// Check if any open map has unsaved changes
    #[allow(dead_code)]
    pub fn has_any_unsaved(&self) -> bool {
        self.maps.values().any(|m| m.is_dirty)
    }

    /// Get list of maps with unsaved changes
    #[allow(dead_code)]
    pub fn unsaved_maps(&self) -> Vec<&OpenMap> {
        self.maps.values().filter(|m| m.is_dirty).collect()
    }
}

/// UI state for unsaved changes confirmation dialogs
#[derive(Resource, Default)]
pub struct UnsavedChangesDialog {
    /// Show dialog for switching maps
    #[allow(dead_code)]
    pub show_switch_confirmation: bool,
    /// The map ID we want to switch to
    #[allow(dead_code)]
    pub pending_switch_id: Option<u64>,
    /// Show dialog for closing app
    pub show_close_confirmation: bool,
    /// Show dialog for loading a new map
    #[allow(dead_code)]
    pub show_load_confirmation: bool,
    /// Path to load after confirmation
    #[allow(dead_code)]
    pub pending_load_path: Option<PathBuf>,
}

#[allow(clippy::too_many_arguments)]
pub fn save_map_system(
    mut events: MessageReader<SaveMapRequest>,
    map_data: Res<MapData>,
    placed_items: Query<(&PlacedItem, &Transform)>,
    paths: Query<&DrawnPath>,
    lines: Query<&DrawnLine>,
    texts: Query<(&Transform, &TextAnnotation)>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut config_events: MessageWriter<UpdateLastMapPath>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
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
                    // Update current map file and config
                    current_map_file.path = Some(event.path.clone());
                    config_events.write(UpdateLastMapPath {
                        path: event.path.clone(),
                    });

                    // Clear dirty state
                    dirty_state.is_dirty = false;
                    dirty_state.last_known_item_count = placed_items.iter().count();
                    dirty_state.last_known_annotation_count =
                        paths.iter().count() + lines.iter().count() + texts.iter().count();

                    // Update open maps
                    if let Some(active_map) = open_maps.active_map_mut() {
                        active_map.is_dirty = false;
                        active_map.path = Some(event.path.clone());
                        active_map.name = event
                            .path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                    }
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
    mut current_map_file: ResMut<CurrentMapFile>,
    mut config_events: MessageWriter<UpdateLastMapPath>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
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
            // Clamp z_index to valid range (for migration from old maps with larger ranges)
            let z_index = item.z_index.clamp(0, Layer::max_z_index());
            let z = item.layer.z_base() + z_index as f32;

            // Items on non-player-visible layers (GM, FogOfWar) go to render layer 1 (editor-only)
            let render_layer = if item.layer.is_player_visible() {
                RenderLayers::layer(0)
            } else {
                RenderLayers::layer(1)
            };

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
                    z_index,
                },
                render_layer,
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

        // Update current map file and config
        current_map_file.path = Some(event.path.clone());
        config_events.write(UpdateLastMapPath {
            path: event.path.clone(),
        });

        // Clear dirty state (freshly loaded map is clean)
        dirty_state.is_dirty = false;
        dirty_state.last_known_item_count = 0; // Will be updated by detection system
        dirty_state.last_known_annotation_count = 0;

        // Update open maps - create new entry for this map
        let map_name = event
            .path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let new_id = open_maps.next_id;
        open_maps.next_id += 1;

        open_maps.maps.insert(
            new_id,
            OpenMap {
                id: new_id,
                name: map_name,
                path: Some(event.path.clone()),
                is_dirty: false,
                saved_state: None,
            },
        );
        open_maps.active_map_id = Some(new_id);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn new_map_system(
    mut commands: Commands,
    mut events: MessageReader<NewMapRequest>,
    mut map_data: ResMut<MapData>,
    existing_items: Query<Entity, With<PlacedItem>>,
    existing_annotations: Query<Entity, With<AnnotationMarker>>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
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

        // Clear current map file (new map has no file yet)
        current_map_file.path = None;

        // Clear dirty state
        dirty_state.is_dirty = false;
        dirty_state.last_known_item_count = 0;
        dirty_state.last_known_annotation_count = 0;

        // Create new entry in open maps
        let new_id = open_maps.next_id;
        open_maps.next_id += 1;

        open_maps.maps.insert(
            new_id,
            OpenMap {
                id: new_id,
                name: "Untitled Map".to_string(),
                path: None,
                is_dirty: false,
                saved_state: None,
            },
        );
        open_maps.active_map_id = Some(new_id);

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

/// System that detects changes to the map and marks it as dirty
pub fn detect_map_changes(
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
    placed_items: Query<Entity, With<PlacedItem>>,
    annotations: Query<Entity, With<AnnotationMarker>>,
) {
    let current_item_count = placed_items.iter().count();
    let current_annotation_count = annotations.iter().count();

    // Check if counts changed
    let count_changed = current_item_count != dirty_state.last_known_item_count
        || current_annotation_count != dirty_state.last_known_annotation_count;

    if count_changed {
        dirty_state.is_dirty = true;
        dirty_state.last_known_item_count = current_item_count;
        dirty_state.last_known_annotation_count = current_annotation_count;

        // Update the active map's dirty state
        if let Some(active_map) = open_maps.active_map_mut() {
            active_map.is_dirty = true;
        }
    }
}

/// Helper to capture current map state as a SavedMap
fn capture_current_map_state(
    map_data: &MapData,
    placed_items: &Query<(&PlacedItem, &Transform)>,
    paths: &Query<&DrawnPath>,
    lines: &Query<&DrawnLine>,
    texts: &Query<(&Transform, &TextAnnotation)>,
) -> SavedMap {
    let items: Vec<SavedPlacedItem> = placed_items
        .iter()
        .map(|(item, transform)| SavedPlacedItem::from_entity(item, transform))
        .collect();

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

    SavedMap {
        map_data: map_data.clone(),
        placed_items: items,
        annotations: SavedAnnotations {
            paths: saved_paths,
            lines: saved_lines,
            text_boxes: saved_texts,
        },
    }
}

/// System to handle switching between open maps
#[allow(clippy::too_many_arguments)]
pub fn switch_map_system(
    mut commands: Commands,
    mut events: MessageReader<SwitchMapRequest>,
    mut map_data: ResMut<MapData>,
    mut open_maps: ResMut<OpenMaps>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut dirty_state: ResMut<MapDirtyState>,
    asset_server: Res<AssetServer>,
    placed_items_query: Query<(&PlacedItem, &Transform)>,
    existing_items: Query<Entity, With<PlacedItem>>,
    existing_annotations: Query<Entity, With<AnnotationMarker>>,
    paths: Query<&DrawnPath>,
    lines: Query<&DrawnLine>,
    texts: Query<(&Transform, &TextAnnotation)>,
) {
    for event in events.read() {
        let target_id = event.map_id;

        // Don't switch to the already active map
        if open_maps.active_map_id == Some(target_id) {
            continue;
        }

        // First, save the current map state
        if let Some(current_id) = open_maps.active_map_id {
            let current_state =
                capture_current_map_state(&map_data, &placed_items_query, &paths, &lines, &texts);
            let current_dirty = dirty_state.is_dirty;

            if let Some(current_map) = open_maps.maps.get_mut(&current_id) {
                current_map.saved_state = Some(current_state);
                current_map.is_dirty = current_dirty;
            }
        }

        // Now load the target map
        if let Some(target_map) = open_maps.maps.get(&target_id).cloned() {
            // Clear existing items
            for entity in existing_items.iter() {
                commands.entity(entity).despawn();
            }

            // Clear existing annotations
            for entity in existing_annotations.iter() {
                commands.entity(entity).despawn();
            }

            // Load target map state
            if let Some(saved_state) = &target_map.saved_state {
                // Restore map data
                *map_data = saved_state.map_data.clone();

                // Spawn placed items
                for item in &saved_state.placed_items {
                    let texture: Handle<Image> = asset_server.load(&item.asset_path);
                    let z_index = item.z_index.clamp(0, Layer::max_z_index());
                    let z = item.layer.z_base() + z_index as f32;

                    let render_layer = if item.layer.is_player_visible() {
                        RenderLayers::layer(0)
                    } else {
                        RenderLayers::layer(1)
                    };

                    commands.spawn((
                        Sprite::from_image(texture),
                        Transform {
                            translation: item.position.extend(z),
                            rotation: Quat::from_rotation_z(item.rotation),
                            scale: item.scale.extend(1.0),
                        },
                        PlacedItem {
                            asset_path: item.asset_path.clone(),
                            layer: item.layer,
                            z_index,
                        },
                        render_layer,
                    ));
                }

                // Spawn annotations
                let z = Layer::Annotation.z_base();

                for path in &saved_state.annotations.paths {
                    commands.spawn((
                        Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                        DrawnPath {
                            points: path.points.clone(),
                            color: array_to_color(path.color),
                            stroke_width: path.stroke_width,
                        },
                        AnnotationMarker,
                    ));
                }

                for line in &saved_state.annotations.lines {
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

                for text in &saved_state.annotations.text_boxes {
                    commands.spawn((
                        Transform::from_translation(text.position.extend(z)),
                        TextAnnotation {
                            content: text.content.clone(),
                            font_size: text.font_size,
                            color: array_to_color(text.color),
                        },
                        AnnotationMarker,
                    ));
                }
            } else {
                // No saved state, start with empty/default map
                *map_data = MapData::default();
                map_data.name = target_map.name.clone();
            }

            // Update current map file
            current_map_file.path = target_map.path.clone();

            // Update dirty state
            dirty_state.is_dirty = target_map.is_dirty;
            dirty_state.last_known_item_count = 0; // Will be updated by detect_map_changes
            dirty_state.last_known_annotation_count = 0;

            // Update active map
            open_maps.active_map_id = Some(target_id);

            info!("Switched to map: {}", target_map.name);
        }
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
