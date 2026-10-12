//! State resources for tracking annotation tool state.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::map::MapData;

use super::super::history::MapObject;

#[derive(Resource, Default)]
pub struct DrawState {
    pub is_drawing: bool,
    pub current_points: Vec<Vec2>,
}

#[derive(Resource, Default)]
pub struct LineDrawState {
    pub start_point: Option<Vec2>,
}

/// The note being typed with the Text tool
#[derive(Resource, Default)]
pub struct TextEditState {
    pub editing_entity: Option<Entity>,
    pub text_buffer: String,
    /// The note as it was before this edit; None for a new note
    pub before: Option<MapObject>,
    /// Set by another system to end the edit (true keeps the typed text)
    pub finish: Option<bool>,
}

#[derive(Resource)]
pub struct AnnotationSettings {
    pub stroke_color: Color,
    pub stroke_width: f32,
    pub font_size: f32,
}

impl Default for AnnotationSettings {
    fn default() -> Self {
        Self {
            stroke_color: Color::srgb(1.0, 0.0, 0.0),
            stroke_width: 3.0,
            font_size: 24.0,
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct AnnotationResources<'w> {
    pub mouse_button: Res<'w, ButtonInput<MouseButton>>,
    pub draw_state: ResMut<'w, DrawState>,
    pub settings: Res<'w, AnnotationSettings>,
    pub map_data: Res<'w, MapData>,
}
