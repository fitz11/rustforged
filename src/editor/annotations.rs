use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;
use serde::{Deserialize, Serialize};

use super::camera::EditorCamera;
use super::tools::{CurrentTool, EditorTool};
use crate::map::Layer;

// ============================================================================
// Components
// ============================================================================

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

#[derive(Component)]
pub struct EditingText;

// ============================================================================
// State Resources
// ============================================================================

#[derive(Resource, Default)]
pub struct DrawState {
    pub is_drawing: bool,
    pub current_points: Vec<Vec2>,
}

#[derive(Resource, Default)]
pub struct LineDrawState {
    pub start_point: Option<Vec2>,
}

#[derive(Resource, Default)]
pub struct TextEditState {
    pub editing_entity: Option<Entity>,
    pub text_buffer: String,
    pub cursor_position: Vec2,
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
    mouse_button: Res<'w, ButtonInput<MouseButton>>,
    current_tool: Res<'w, CurrentTool>,
    draw_state: ResMut<'w, DrawState>,
    settings: Res<'w, AnnotationSettings>,
}

// ============================================================================
// Systems
// ============================================================================

pub fn handle_draw(
    mut commands: Commands,
    mut res: AnnotationResources,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut contexts: EguiContexts,
) {
    if res.current_tool.tool != EditorTool::Draw {
        // If we were drawing but switched tools, finalize
        if res.draw_state.is_drawing && res.draw_state.current_points.len() >= 2 {
            spawn_drawn_path(&mut commands, &res.draw_state, &res.settings);
        }
        res.draw_state.is_drawing = false;
        res.draw_state.current_points.clear();
        return;
    }

    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.is_pointer_over_area()
    {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    if res.mouse_button.just_pressed(MouseButton::Left) {
        res.draw_state.is_drawing = true;
        res.draw_state.current_points.clear();
        res.draw_state.current_points.push(world_pos);
    } else if res.mouse_button.pressed(MouseButton::Left) && res.draw_state.is_drawing {
        // Add point if it's far enough from the last one (reduces point count)
        if let Some(last) = res.draw_state.current_points.last() {
            if world_pos.distance(*last) > 2.0 {
                res.draw_state.current_points.push(world_pos);
            }
        }
    } else if res.mouse_button.just_released(MouseButton::Left) && res.draw_state.is_drawing {
        res.draw_state.is_drawing = false;
        if res.draw_state.current_points.len() >= 2 {
            spawn_drawn_path(&mut commands, &res.draw_state, &res.settings);
        }
        res.draw_state.current_points.clear();
    }
}

fn spawn_drawn_path(
    commands: &mut Commands,
    draw_state: &DrawState,
    settings: &AnnotationSettings,
) {
    let z = Layer::Annotation.z_base();
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, 0.0, z)),
        DrawnPath {
            points: draw_state.current_points.clone(),
            color: settings.stroke_color,
            stroke_width: settings.stroke_width,
        },
        AnnotationMarker,
    ));
}

pub fn handle_line(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    current_tool: Res<CurrentTool>,
    mut line_state: ResMut<LineDrawState>,
    settings: Res<AnnotationSettings>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut contexts: EguiContexts,
) {
    if current_tool.tool != EditorTool::Line {
        line_state.start_point = None;
        return;
    }

    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.is_pointer_over_area() {
            return;
        }
    }

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(start) = line_state.start_point {
            // Second click - create line
            let z = Layer::Annotation.z_base();
            commands.spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                DrawnLine {
                    start,
                    end: world_pos,
                    color: settings.stroke_color,
                    stroke_width: settings.stroke_width,
                },
                AnnotationMarker,
            ));
            line_state.start_point = None;
        } else {
            // First click - set start point
            line_state.start_point = Some(world_pos);
        }
    }

    // Right click cancels
    if mouse_button.just_pressed(MouseButton::Right) {
        line_state.start_point = None;
    }
}

pub fn handle_text(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    current_tool: Res<CurrentTool>,
    mut text_state: ResMut<TextEditState>,
    settings: Res<AnnotationSettings>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut contexts: EguiContexts,
    text_query: Query<(Entity, &Transform, &TextAnnotation), Without<EditingText>>,
    mut editing_query: Query<(Entity, &mut TextAnnotation), With<EditingText>>,
) {
    if current_tool.tool != EditorTool::Text {
        // Finalize any editing text when switching away
        if text_state.editing_entity.is_some() {
            for (entity, mut text) in editing_query.iter_mut() {
                text.content = text_state.text_buffer.clone();
                commands.entity(entity).remove::<EditingText>();
            }
            text_state.editing_entity = None;
            text_state.text_buffer.clear();
        }
        return;
    }

    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.is_pointer_over_area() {
            return;
        }
    }

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Left) {
        // Check if clicking on existing text
        let mut clicked_text = None;
        for (entity, transform, _text) in text_query.iter() {
            let pos = transform.translation.truncate();
            let half_size = Vec2::new(100.0, 20.0); // Approximate text bounds
            if (world_pos.x - pos.x).abs() < half_size.x
                && (world_pos.y - pos.y).abs() < half_size.y
            {
                clicked_text = Some(entity);
                break;
            }
        }

        if let Some(entity) = clicked_text {
            // Edit existing text
            if let Ok((_, _, text)) = text_query.get(entity) {
                text_state.editing_entity = Some(entity);
                text_state.text_buffer = text.content.clone();
                commands.entity(entity).insert(EditingText);
            }
        } else {
            // Finalize any current editing
            if text_state.editing_entity.is_some() {
                for (entity, mut text) in editing_query.iter_mut() {
                    text.content = text_state.text_buffer.clone();
                    commands.entity(entity).remove::<EditingText>();
                }
            }

            // Create new text at position
            let z = Layer::Annotation.z_base();
            let entity = commands
                .spawn((
                    Transform::from_translation(world_pos.extend(z)),
                    TextAnnotation {
                        content: String::new(),
                        font_size: settings.font_size,
                        color: settings.stroke_color,
                    },
                    AnnotationMarker,
                    EditingText,
                ))
                .id();
            text_state.editing_entity = Some(entity);
            text_state.text_buffer.clear();
            text_state.cursor_position = world_pos;
        }
    }
}

// ============================================================================
// Rendering
// ============================================================================

pub fn render_drawn_paths(mut gizmos: Gizmos, paths: Query<&DrawnPath>) {
    for path in paths.iter() {
        if path.points.len() < 2 {
            continue;
        }

        for window in path.points.windows(2) {
            gizmos.line_2d(window[0], window[1], path.color);
        }
    }
}

pub fn render_drawn_lines(mut gizmos: Gizmos, lines: Query<&DrawnLine>) {
    for line in lines.iter() {
        gizmos.line_2d(line.start, line.end, line.color);
    }
}

pub fn render_line_preview(
    mut gizmos: Gizmos,
    current_tool: Res<CurrentTool>,
    line_state: Res<LineDrawState>,
    settings: Res<AnnotationSettings>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
) {
    if current_tool.tool != EditorTool::Line {
        return;
    }

    let Some(start) = line_state.start_point else {
        return;
    };

    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Draw preview line with lower opacity
    let preview_color = settings.stroke_color.with_alpha(0.5);
    gizmos.line_2d(start, world_pos, preview_color);
}

pub fn render_draw_preview(
    mut gizmos: Gizmos,
    current_tool: Res<CurrentTool>,
    draw_state: Res<DrawState>,
    settings: Res<AnnotationSettings>,
) {
    if current_tool.tool != EditorTool::Draw || !draw_state.is_drawing {
        return;
    }

    if draw_state.current_points.len() < 2 {
        return;
    }

    for window in draw_state.current_points.windows(2) {
        gizmos.line_2d(window[0], window[1], settings.stroke_color);
    }
}

pub fn render_text_annotations(mut gizmos: Gizmos, texts: Query<(&Transform, &TextAnnotation)>) {
    for (transform, text) in texts.iter() {
        let pos = transform.translation.truncate();

        // Draw a simple box around text location
        // Actual text rendering will use Text2d or egui
        let width = text.content.len() as f32 * text.font_size * 0.5;
        let height = text.font_size;
        let half_size = Vec2::new(width.max(20.0) / 2.0, height / 2.0);

        gizmos.rect_2d(
            Isometry2d::from_translation(pos),
            half_size * 2.0,
            text.color.with_alpha(0.3),
        );
    }
}
