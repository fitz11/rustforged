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
//! - [`SelectedObjects`]: The selected items and annotations, for delete, cut and copy
//!
//! ## Helper Functions
//!
//! - [`is_cursor_over_ui`]: Check if cursor is over egui UI (for input gating)

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::map::{ItemId, PlacedItem, Selected};
use crate::ui::panel_layout::is_pointer_over_ui;

use super::EditorCamera;
use super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use super::history::MapObject;

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

    /// Where the editor camera is looking (the middle of the view)
    pub fn camera_world_pos(&self) -> Option<Vec2> {
        let (_, transform) = self.camera.single().ok()?;
        Some(transform.translation().truncate())
    }
}

/// Bundled camera queries including projection (for zoom-aware operations)
#[derive(SystemParam)]
pub struct CameraWithProjection<'w, 's> {
    pub window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    pub camera: Query<
        'w,
        's,
        (
            &'static Camera,
            &'static GlobalTransform,
            &'static Projection,
        ),
        With<EditorCamera>,
    >,
}

impl CameraWithProjection<'_, '_> {
    /// Get the world position of the cursor, if available
    pub fn cursor_world_pos(&self) -> Option<Vec2> {
        let window = self.window.single().ok()?;
        let (camera, transform, _) = self.camera.single().ok()?;
        let cursor_pos = window.cursor_position()?;
        camera.viewport_to_world_2d(transform, cursor_pos).ok()
    }

    /// The world area the editor camera shows, as (min, max)
    pub fn visible_world_rect(&self) -> Option<(Vec2, Vec2)> {
        let (_, transform, projection) = self.camera.single().ok()?;
        let Projection::Orthographic(ortho) = projection else {
            return None;
        };
        // The projection's area already includes the zoom
        let center = transform.translation().truncate();
        Some((center + ortho.area.min, center + ortho.area.max))
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
    pub texts: Query<
        'w,
        's,
        (Entity, &'static Transform, &'static TextAnnotation),
        With<AnnotationMarker>,
    >,
}

/// The selected placed items and annotations, captured as the undo history remembers them
/// (used to delete, cut and copy the selection)
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub struct SelectedObjects<'w, 's> {
    items: Query<
        'w,
        's,
        (
            Entity,
            &'static ItemId,
            &'static PlacedItem,
            &'static Transform,
        ),
        With<Selected>,
    >,
    paths: Query<
        'w,
        's,
        (Entity, &'static ItemId, &'static DrawnPath),
        (With<Selected>, With<AnnotationMarker>),
    >,
    lines: Query<
        'w,
        's,
        (Entity, &'static ItemId, &'static DrawnLine),
        (With<Selected>, With<AnnotationMarker>),
    >,
    texts: Query<
        'w,
        's,
        (
            Entity,
            &'static ItemId,
            &'static TextAnnotation,
            &'static Transform,
        ),
        (With<Selected>, With<AnnotationMarker>),
    >,
}

impl SelectedObjects<'_, '_> {
    /// Every selected thing on the map, with its entity.
    pub fn collect(&self) -> Vec<(Entity, MapObject)> {
        let items = self.items.iter().map(|(entity, id, item, transform)| {
            (entity, MapObject::item(*id, item.clone(), *transform))
        });
        let paths = self.paths.iter().map(|(entity, id, path)| {
            (
                entity,
                MapObject::Path {
                    id: *id,
                    path: path.clone(),
                },
            )
        });
        let lines = self.lines.iter().map(|(entity, id, line)| {
            (
                entity,
                MapObject::Line {
                    id: *id,
                    line: line.clone(),
                },
            )
        });
        let texts = self.texts.iter().map(|(entity, id, text, transform)| {
            (
                entity,
                MapObject::Text {
                    id: *id,
                    text: text.clone(),
                    position: transform.translation.truncate(),
                },
            )
        });
        items.chain(paths).chain(lines).chain(texts).collect()
    }
}

/// The area the map's placed items cover (min, max), for revealing all its fog or framing
/// it; None for an empty map. Kept up to date by [`update_map_extent`], so systems that
/// also edit items' transforms (the Properties panel) can read it without a query conflict.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct MapExtent(pub Option<(Vec2, Vec2)>);

/// Recomputes [`MapExtent`] from the placed items.
pub fn update_map_extent(
    items: Query<(&Transform, &Sprite), With<PlacedItem>>,
    images: Res<Assets<Image>>,
    mut extent: ResMut<MapExtent>,
) {
    let bounds = items
        .iter()
        .map(|(transform, sprite)| super::selection::item_world_bounds(transform, sprite, &images))
        .reduce(|(min_a, max_a), (min_b, max_b)| (min_a.min(min_b), max_a.max(max_b)));
    extent.set_if_neq(MapExtent(bounds));
}

/// Check if the cursor is over egui UI (docked panels, windows, or areas)
pub fn is_cursor_over_ui(contexts: &mut EguiContexts) -> bool {
    contexts.ctx_mut().is_ok_and(|ctx| is_pointer_over_ui(ctx))
}
