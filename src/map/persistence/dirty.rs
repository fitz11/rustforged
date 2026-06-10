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
    // Ignore the spawn wave from a load/new/switch. Running the system still
    // advances its change-detection tick, so these adds won't be seen later.
    if dirty_state.suppress_detection > 0 {
        return;
    }

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
    // Ignore the despawn wave from a load/new/switch. Drain the buffers so the
    // events don't leak into a later frame once suppression ends.
    if dirty_state.suppress_detection > 0 {
        removed_items.clear();
        removed_annotations.clear();
        return;
    }

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
    // Ignore the transform "changes" from newly spawned items after a
    // load/new/switch (a freshly added Transform also counts as Changed).
    if dirty_state.suppress_detection > 0 {
        return;
    }

    // Only run if transforms changed
    if changed_items.is_empty() {
        return;
    }

    dirty_state.is_dirty = true;
    if let Some(active_map) = open_maps.active_map_mut() {
        active_map.is_dirty = true;
    }
}

/// System that ticks down the change-detection suppression window. Runs after
/// the detection systems each frame so they observe the current value first.
pub fn decay_dirty_suppression(mut dirty_state: ResMut<MapDirtyState>) {
    if dirty_state.suppress_detection > 0 {
        dirty_state.suppress_detection -= 1;
    }
}
