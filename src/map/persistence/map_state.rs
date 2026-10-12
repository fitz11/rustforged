//! Map state management: new map, switch map, and state capture.

use bevy::prelude::*;

use crate::editor::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use crate::map::{
    FogOfWarData, MAP_FORMAT_VERSION, MapData, PlacedItem, SavedAnnotations, SavedFogOfWar,
    SavedLine, SavedMap, SavedPath, SavedPlacedItem, SavedTextBox, Stacker,
};

use super::helpers::{array_to_color, color_to_array};
use super::messages::{MapReplaced, NewMapRequest, SwitchMapRequest};
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
    mut replaced: MessageWriter<MapReplaced>,
) {
    for _ in events.read() {
        replaced.write(MapReplaced);
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
        // Ignore the despawn wave so the freshly-cleared map stays clean.
        dirty_state.suppress_change_detection();

        // Replace the current active map entry (consistent with load_map_system).
        // "New Map" discards the current map rather than leaving an orphaned,
        // stateless tab in the open-maps list.
        if let Some(active_id) = open_maps.active_map_id {
            open_maps.maps.remove(&active_id);
        }

        // Create new entry in open maps
        let new_id = open_maps.next_id;
        open_maps.next_id += 1;

        open_maps.maps.insert(
            new_id,
            OpenMap {
                id: new_id,
                name: "Untitled Map".to_string(),
                path: None,
                stashed_dirty: false,
                saved_state: None,
                map_id: None,
                disk_hash: None,
                asset_fingerprints: Default::default(),
            },
        );
        open_maps.active_map_id = Some(new_id);

        info!("Created new map");
    }
}

/// Helper to capture current map state as a SavedMap
pub(super) fn capture_current_map_state(
    map_data: &MapData,
    fog_data: &FogOfWarData,
    placed_items: &Query<(&PlacedItem, &Transform)>,
    paths: &Query<&DrawnPath>,
    lines: &Query<&DrawnLine>,
    texts: &Query<(&Transform, &TextAnnotation)>,
) -> SavedMap {
    // In stacking order, bottom first, so loading the file stacks them the same way
    let mut stacked: Vec<_> = placed_items.iter().collect();
    stacked.sort_by(|(a, at), (b, bt)| {
        at.translation.z.total_cmp(&bt.translation.z).then_with(|| {
            SavedPlacedItem::from_entity(a, at).canonical_cmp(&SavedPlacedItem::from_entity(b, bt))
        })
    });
    let items: Vec<SavedPlacedItem> = stacked
        .into_iter()
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

    // Id and fingerprints live on the OpenMap entry; this snapshot only holds what's on the map
    SavedMap {
        format_version: MAP_FORMAT_VERSION,
        map_id: None,
        saved_by: None,
        asset_fingerprints: Default::default(),
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

/// Spawns the placed items and annotations of `map`.
pub(super) fn spawn_map_contents(
    commands: &mut Commands,
    asset_server: &AssetServer,
    map: &SavedMap,
) {
    // The file lists items in stacking order, bottom first
    let mut stacker = Stacker::default();
    for saved in &map.placed_items {
        let item = saved.placed_item();
        let mut transform = saved.transform();
        transform.translation.z = stacker.next_z(&item);
        commands.spawn(item.bundle(asset_server, transform));
    }

    for path in &map.annotations.paths {
        commands.spawn(
            DrawnPath {
                points: path.points.clone(),
                color: array_to_color(path.color),
                stroke_width: path.stroke_width,
            }
            .bundle(),
        );
    }

    for line in &map.annotations.lines {
        commands.spawn(
            DrawnLine {
                start: line.start,
                end: line.end,
                color: array_to_color(line.color),
                stroke_width: line.stroke_width,
            }
            .bundle(),
        );
    }

    for text in &map.annotations.text_boxes {
        commands.spawn(
            TextAnnotation {
                content: text.content.clone(),
                font_size: text.font_size,
                color: array_to_color(text.color),
            }
            .bundle(text.position),
        );
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
    mut replaced: MessageWriter<MapReplaced>,
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
                current_map.stashed_dirty = current_dirty;
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

                spawn_map_contents(&mut commands, &asset_server, saved_state);
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
            dirty_state.is_dirty = target_map.stashed_dirty;
            // Ignore the despawn/respawn wave from switching maps.
            dirty_state.suppress_change_detection();

            // Update active map
            open_maps.active_map_id = Some(target_id);

            replaced.write(MapReplaced);
            info!("Switched to map: {}", target_map.name);
        }
    }
}
