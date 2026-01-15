//! Component types for annotation entities.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct DrawnPath {
    pub points: Vec<Vec2>,
    pub color: Color,
    pub stroke_width: f32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct DrawnLine {
    pub start: Vec2,
    pub end: Vec2,
    pub color: Color,
    pub stroke_width: f32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    pub content: String,
    pub font_size: f32,
    pub color: Color,
}

#[derive(Component)]
pub struct AnnotationMarker;

// Text tool disabled - see TODO in tools.rs
#[allow(dead_code)]
#[derive(Component)]
pub struct EditingText;
