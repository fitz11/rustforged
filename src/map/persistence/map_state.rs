//! Map state management: new map, switch map, and state capture.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::editor::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use crate::map::{
    AssetManifest, FogOfWarData, Layer, MapData, PlacedItem, SavedAnnotations, SavedFogOfWar,
    SavedLine, SavedMap, SavedPath, SavedPlacedItem, SavedTextBox,
};

use super::helpers::{array_to_color, color_to_array};
use super::messages::{NewMapRequest, SwitchMapRequest};
use super::resources::{CurrentMapFile, MapDirtyState, OpenMap, OpenMaps};

#[allow(clippy::too_many_arguments)]
pub fn new_map_system(
    mut commands: Commands,
    mut events: MessageReader<NewMapRequest>,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
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

        // Reset fog of war to default (empty = fully fogged)
        *fog_data = FogOfWarData::default();

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

/// Helper to capture current map state as a SavedMap
fn capture_current_map_state(
    map_data: &MapData,
    fog_data: &FogOfWarData,
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

    let asset_manifest = AssetManifest::from_items(items.iter());

    SavedMap {
        asset_manifest,
        map_data: map_data.clone(),
        placed_items: items,
        annotations: SavedAnnotations {
            paths: saved_paths,
            lines: saved_lines,
            text_boxes: saved_texts,
        },
        fog_of_war: SavedFogOfWar::from(fog_data),
    }
}

/// System to handle switching between open maps
#[allow(clippy::too_many_arguments)]
pub fn switch_map_system(
    mut commands: Commands,
    mut events: MessageReader<SwitchMapRequest>,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
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
            let current_state = capture_current_map_state(
                &map_data,
                &fog_data,
                &placed_items_query,
                &paths,
                &lines,
                &texts,
            );
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

                // Restore fog of war data
                *fog_data = FogOfWarData::from(saved_state.fog_of_war.clone());

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
                // Reset fog of war to default (empty = fully fogged)
                *fog_data = FogOfWarData::default();
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
