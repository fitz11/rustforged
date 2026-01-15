//! Data types for serializing editor state in undo/redo commands.

use bevy::prelude::*;

use crate::map::Layer;

use super::super::annotations::{DrawnLine, DrawnPath};

/// Serializable data for a placed item
#[derive(Clone, Debug)]
pub struct PlacedItemData {
    pub entity: Entity,
    pub asset_path: String,
    pub layer: Layer,
    pub z_index: i32,
    pub transform: TransformData,
}

/// Serializable transform data
#[derive(Clone, Debug, Copy)]
pub struct TransformData {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl From<&Transform> for TransformData {
    fn from(t: &Transform) -> Self {
        Self {
            translation: t.translation,
            rotation: t.rotation,
            scale: t.scale,
        }
    }
}

impl From<TransformData> for Transform {
    fn from(t: TransformData) -> Self {
        Transform {
            translation: t.translation,
            rotation: t.rotation,
            scale: t.scale,
        }
    }
}

/// Serializable data for a drawn path
#[derive(Clone, Debug)]
pub struct PathData {
    pub points: Vec<Vec2>,
    pub color: Color,
    pub stroke_width: f32,
}

impl From<&DrawnPath> for PathData {
    fn from(p: &DrawnPath) -> Self {
        Self {
            points: p.points.clone(),
            color: p.color,
            stroke_width: p.stroke_width,
        }
    }
}

/// Serializable data for a drawn line
#[derive(Clone, Debug)]
pub struct LineData {
    pub start: Vec2,
    pub end: Vec2,
    pub color: Color,
    pub stroke_width: f32,
}

impl From<&DrawnLine> for LineData {
    fn from(l: &DrawnLine) -> Self {
        Self {
            start: l.start,
            end: l.end,
            color: l.color,
            stroke_width: l.stroke_width,
        }
    }
}

/// Serializable data for a text annotation
#[derive(Clone, Debug)]
pub struct TextData {
    pub text: String,
    pub position: Vec2,
    pub color: Color,
    pub font_size: f32,
}
