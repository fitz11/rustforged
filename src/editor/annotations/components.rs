//! Component types for annotation entities.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::map::{ItemId, Layer};

/// Annotations are drawn at the Annotation layer's depth; paths and lines keep their
/// points in world space, so their transform stays at the origin.
fn annotation_transform(position: Vec2) -> Transform {
    Transform::from_translation(position.extend(Layer::Annotation.z_base()))
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawnPath {
    pub points: Vec<Vec2>,
    pub color: Color,
    pub stroke_width: f32,
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawnLine {
    pub start: Vec2,
    pub end: Vec2,
    pub color: Color,
    pub stroke_width: f32,
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextAnnotation {
    pub content: String,
    pub font_size: f32,
    pub color: Color,
}

#[derive(Component)]
#[require(ItemId = ItemId::new())]
pub struct AnnotationMarker;

impl DrawnPath {
    /// Components for a freehand path entity.
    pub fn bundle(self) -> impl Bundle {
        (annotation_transform(Vec2::ZERO), self, AnnotationMarker)
    }
}

impl DrawnLine {
    /// Components for a line entity.
    pub fn bundle(self) -> impl Bundle {
        (annotation_transform(Vec2::ZERO), self, AnnotationMarker)
    }
}

impl TextAnnotation {
    /// Components for a text annotation entity at `position`.
    pub fn bundle(self, position: Vec2) -> impl Bundle {
        (annotation_transform(position), self, AnnotationMarker)
    }
}

/// Marks the note being typed with the Text tool
#[derive(Component)]
pub struct EditingText;
