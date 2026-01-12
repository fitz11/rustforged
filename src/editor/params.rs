//! Common SystemParam bundles to reduce parameter counts in editor systems.
//!
//! Bevy ECS systems have a limit of 16 parameters. Complex editor systems often need
//! access to multiple related queries (e.g., camera, window, projection for cursor handling,
//! or multiple annotation types for selection). Rather than hitting this limit, we bundle
//! related queries into SystemParam structs that provide convenient methods.
//!
//! ## Available Bundles
//!
//! - [`CameraParams`]: Basic camera and window access for cursor-to-world conversion
//! - [`CameraWithProjection`]: Extended camera access including zoom scale
//! - [`AnnotationQueries`]: Read-only access to all annotation types (paths, lines, text)
//! - [`SelectedAnnotationQueries`]: Access to selected annotations only
//!
//! ## Helper Functions
//!
//! - [`is_cursor_over_ui`]: Check if cursor is over egui UI (for input gating)

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::map::Selected;

use super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use super::EditorCamera;

/// Bundled camera and window queries for cursor-to-world calculations
#[derive(SystemParam)]
pub struct CameraParams<'w, 's> {
    pub window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    pub camera: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<EditorCamera>>,
}

impl CameraParams<'_, '_> {
    /// Get the world position of the cursor, if available
    pub fn cursor_world_pos(&self) -> Option<Vec2> {
        let window = self.window.single().ok()?;
        let (camera, transform) = self.camera.single().ok()?;
        let cursor_pos = window.cursor_position()?;
        camera.viewport_to_world_2d(transform, cursor_pos).ok()
    }
}

/// Bundled camera queries including projection (for zoom-aware operations)
#[derive(SystemParam)]
pub struct CameraWithProjection<'w, 's> {
    pub window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    pub camera:
        Query<'w, 's, (&'static Camera, &'static GlobalTransform, &'static Projection), With<EditorCamera>>,
}

impl CameraWithProjection<'_, '_> {
    /// Get the world position of the cursor, if available
    pub fn cursor_world_pos(&self) -> Option<Vec2> {
        let window = self.window.single().ok()?;
        let (camera, transform, _) = self.camera.single().ok()?;
        let cursor_pos = window.cursor_position()?;
        camera.viewport_to_world_2d(transform, cursor_pos).ok()
    }

    /// Get the current zoom scale from projection
    pub fn zoom_scale(&self) -> f32 {
        self.camera
            .single()
            .ok()
            .and_then(|(_, _, proj)| {
                if let Projection::Orthographic(ortho) = proj {
                    Some(ortho.scale)
                } else {
                    None
                }
            })
            .unwrap_or(1.0)
    }
}

/// Bundled annotation queries for read-only access
#[derive(SystemParam)]
pub struct AnnotationQueries<'w, 's> {
    pub paths: Query<'w, 's, (Entity, &'static DrawnPath), With<AnnotationMarker>>,
    pub lines: Query<'w, 's, (Entity, &'static DrawnLine), With<AnnotationMarker>>,
    pub texts:
        Query<'w, 's, (Entity, &'static Transform, &'static TextAnnotation), With<AnnotationMarker>>,
}

/// Bundled queries for selected annotations (used in clipboard operations)
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub struct SelectedAnnotationQueries<'w, 's> {
    pub paths: Query<'w, 's, (Entity, &'static DrawnPath), (With<Selected>, With<AnnotationMarker>)>,
    pub lines: Query<'w, 's, (Entity, &'static DrawnLine), (With<Selected>, With<AnnotationMarker>)>,
    pub texts: Query<
        'w,
        's,
        (Entity, &'static Transform, &'static TextAnnotation),
        (With<Selected>, With<AnnotationMarker>),
    >,
}

/// Check if the cursor is over egui UI
pub fn is_cursor_over_ui(contexts: &mut EguiContexts) -> bool {
    contexts
        .ctx_mut()
        .map(|ctx| ctx.is_pointer_over_area())
        .unwrap_or(false)
}
