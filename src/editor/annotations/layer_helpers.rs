//! Helper functions for annotation layer visibility and locking.

use crate::map::{Layer, MapData};

/// Check if the Annotation layer is visible
pub fn is_annotation_layer_visible(map_data: &MapData) -> bool {
    map_data
        .layers
        .iter()
        .find(|ld| ld.layer_type == Layer::Annotation)
        .map(|ld| ld.visible)
        .unwrap_or(true)
}

/// Check if the Annotation layer is locked
pub fn is_annotation_layer_locked(map_data: &MapData) -> bool {
    map_data
        .layers
        .iter()
        .find(|ld| ld.layer_type == Layer::Annotation)
        .map(|ld| ld.locked)
        .unwrap_or(false)
}
