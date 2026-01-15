//! Dirty state detection systems for tracking unsaved changes.

use bevy::prelude::*;

use crate::editor::AnnotationMarker;
use crate::map::PlacedItem;

use super::resources::{MapDirtyState, OpenMaps};

/// System that detects when items are added to the map
pub fn detect_item_additions(
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
    added_items: Query<Entity, Added<PlacedItem>>,
    added_annotations: Query<Entity, Added<AnnotationMarker>>,
) {
    // Only run if something was added
    if added_items.is_empty() && added_annotations.is_empty() {
        return;
    }

    dirty_state.is_dirty = true;
    if let Some(active_map) = open_maps.active_map_mut() {
        active_map.is_dirty = true;
    }
}

/// System that detects when items are removed from the map
pub fn detect_item_removals(
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
    mut removed_items: RemovedComponents<PlacedItem>,
    mut removed_annotations: RemovedComponents<AnnotationMarker>,
) {
    // Only run if something was removed
    if removed_items.read().next().is_none() && removed_annotations.read().next().is_none() {
        return;
    }

    dirty_state.is_dirty = true;
    if let Some(active_map) = open_maps.active_map_mut() {
        active_map.is_dirty = true;
    }
}

/// System that detects when items are transformed (moved, rotated, scaled)
pub fn detect_item_transforms(
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
    changed_items: Query<Entity, (Changed<Transform>, With<PlacedItem>)>,
) {
    // Only run if transforms changed
    if changed_items.is_empty() {
        return;
    }

    dirty_state.is_dirty = true;
    if let Some(active_map) = open_maps.active_map_mut() {
        active_map.is_dirty = true;
    }
}
