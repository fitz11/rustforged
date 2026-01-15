//! Custom gizmo group for annotations (editor-only rendering).

use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;

/// Custom gizmo group for annotations (editor-only rendering)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct AnnotationGizmoGroup;

/// Configure the annotation gizmo group to only render to editor camera (layer 1)
pub fn configure_annotation_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<AnnotationGizmoGroup>();
    // Only render to layer 1 (editor-only, not visible in player view)
    config.render_layers = RenderLayers::layer(1);
}
