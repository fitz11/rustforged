//! Helper functions for annotation layer visibility and locking.

use crate::map::{Layer, MapData};

/// Check if the Annotation layer is visible
pub fn is_annotation_layer_visible(map_data: &MapData) -> bool {
    map_data.is_layer_visible(Layer::Annotation)
}

/// Check if the Annotation layer is locked
pub fn is_annotation_layer_locked(map_data: &MapData) -> bool {
    map_data.is_layer_locked(Layer::Annotation)
}
