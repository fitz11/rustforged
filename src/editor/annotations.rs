use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::gizmos::config::{GizmoConfigGroup, GizmoConfigStore};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;
use serde::{Deserialize, Serialize};

use super::camera::EditorCamera;
use super::tools::{CurrentTool, EditorTool};
use crate::map::{Layer, MapData};

// ============================================================================
// Gizmo Configuration (Editor-Only Rendering)
// ============================================================================

/// Custom gizmo group for annotations (editor-only rendering)
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct AnnotationGizmoGroup;

/// Configure the annotation gizmo group to only render to editor camera (layer 1)
pub fn configure_annotation_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<AnnotationGizmoGroup>();
    // Only render to layer 1 (editor-only, not visible in player view)
    config.render_layers = RenderLayers::layer(1);
}

// ============================================================================
// Layer Helpers
// ============================================================================

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

// ============================================================================
// Hit Testing
// ============================================================================

/// Check if a point is within a given distance of a line segment
fn point_near_segment(point: Vec2, seg_start: Vec2, seg_end: Vec2, threshold: f32) -> bool {
    let line_vec = seg_end - seg_start;
    let line_len_sq = line_vec.length_squared();

    if line_len_sq < 0.0001 {
        // Segment is essentially a point
        return point.distance(seg_start) <= threshold;
    }

    // Project point onto line, clamped to segment
    let t = ((point - seg_start).dot(line_vec) / line_len_sq).clamp(0.0, 1.0);
    let projection = seg_start + line_vec * t;

    point.distance(projection) <= threshold
}

/// Check if a point is near a drawn path
pub fn point_near_path(point: Vec2, path: &DrawnPath) -> bool {
    let threshold = (path.stroke_width * 2.0).max(8.0); // Hit area is at least 8px

    for window in path.points.windows(2) {
        if point_near_segment(point, window[0], window[1], threshold) {
            return true;
        }
    }
    false
}

/// Check if a point is near a drawn line
pub fn point_near_line(point: Vec2, line: &DrawnLine) -> bool {
    let threshold = (line.stroke_width * 2.0).max(8.0);
    point_near_segment(point, line.start, line.end, threshold)
}

/// Check if a point is inside a text annotation's bounding box
pub fn point_in_text(point: Vec2, transform: &Transform, text: &TextAnnotation) -> bool {
    let pos = transform.translation.truncate();
    let width = (text.content.len() as f32 * text.font_size * 0.5).max(40.0);
    let height = text.font_size.max(20.0);
    let half_size = Vec2::new(width / 2.0, height / 2.0);

    (point.x - pos.x).abs() < half_size.x && (point.y - pos.y).abs() < half_size.y
}

/// Get the bounding box of a path (min, max corners)
pub fn path_bounds(path: &DrawnPath) -> (Vec2, Vec2) {
    if path.points.is_empty() {
        return (Vec2::ZERO, Vec2::ZERO);
    }

    let mut min = path.points[0];
    let mut max = path.points[0];

    for &p in &path.points {
        min = min.min(p);
        max = max.max(p);
    }

    // Expand by stroke width
    let padding = path.stroke_width;
    (min - Vec2::splat(padding), max + Vec2::splat(padding))
}

/// Get the bounding box of a line (min, max corners)
pub fn line_bounds(line: &DrawnLine) -> (Vec2, Vec2) {
    let min = line.start.min(line.end);
    let max = line.start.max(line.end);
    let padding = line.stroke_width;
    (min - Vec2::splat(padding), max + Vec2::splat(padding))
}

/// Get the bounding box of a text annotation (min, max corners)
pub fn text_bounds(transform: &Transform, text: &TextAnnotation) -> (Vec2, Vec2) {
    let pos = transform.translation.truncate();
    let width = (text.content.len() as f32 * text.font_size * 0.5).max(40.0);
    let height = text.font_size.max(20.0);
    let half_size = Vec2::new(width / 2.0, height / 2.0);

    (pos - half_size, pos + half_size)
}

/// Check if a path overlaps with a selection rectangle
pub fn path_overlaps_rect(rect_min: Vec2, rect_max: Vec2, path: &DrawnPath) -> bool {
    let (path_min, path_max) = path_bounds(path);
    rect_min.x < path_max.x
        && rect_max.x > path_min.x
        && rect_min.y < path_max.y
        && rect_max.y > path_min.y
}

/// Check if a line overlaps with a selection rectangle
pub fn line_overlaps_rect(rect_min: Vec2, rect_max: Vec2, line: &DrawnLine) -> bool {
    let (line_min, line_max) = line_bounds(line);
    rect_min.x < line_max.x
        && rect_max.x > line_min.x
        && rect_min.y < line_max.y
        && rect_max.y > line_min.y
}

/// Check if a text annotation overlaps with a selection rectangle
pub fn text_overlaps_rect(
    rect_min: Vec2,
    rect_max: Vec2,
    transform: &Transform,
    text: &TextAnnotation,
) -> bool {
    let (text_min, text_max) = text_bounds(transform, text);
    rect_min.x < text_max.x
        && rect_max.x > text_min.x
        && rect_min.y < text_max.y
        && rect_max.y > text_min.y
}

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
    map_data: Res<'w, MapData>,
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

    // Don't allow drawing if annotation layer is locked
    if is_annotation_layer_locked(&res.map_data) {
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
        if let Some(last) = res.draw_state.current_points.last()
            && world_pos.distance(*last) > 2.0
        {
            res.draw_state.current_points.push(world_pos);
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
    map_data: Res<MapData>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut contexts: EguiContexts,
) {
    if current_tool.tool != EditorTool::Line {
        line_state.start_point = None;
        return;
    }

    // Don't allow line drawing if annotation layer is locked
    if is_annotation_layer_locked(&map_data) {
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
    map_data: Res<MapData>,
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

    // Don't allow text creation if annotation layer is locked
    if is_annotation_layer_locked(&map_data) {
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
// Rendering (Editor-Only via AnnotationGizmoGroup)
// ============================================================================

pub fn render_drawn_paths(
    mut gizmos: Gizmos<AnnotationGizmoGroup>,
    paths: Query<&DrawnPath>,
    map_data: Res<MapData>,
) {
    if !is_annotation_layer_visible(&map_data) {
        return;
    }

    for path in paths.iter() {
        if path.points.len() < 2 {
            continue;
        }

        for window in path.points.windows(2) {
            gizmos.line_2d(window[0], window[1], path.color);
        }
    }
}

pub fn render_drawn_lines(
    mut gizmos: Gizmos<AnnotationGizmoGroup>,
    lines: Query<&DrawnLine>,
    map_data: Res<MapData>,
) {
    if !is_annotation_layer_visible(&map_data) {
        return;
    }

    for line in lines.iter() {
        gizmos.line_2d(line.start, line.end, line.color);
    }
}

pub fn render_line_preview(
    mut gizmos: Gizmos<AnnotationGizmoGroup>,
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
    mut gizmos: Gizmos<AnnotationGizmoGroup>,
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

/// Render text annotations using egui (editor-only, doesn't show in player view)
pub fn render_text_annotations(
    mut contexts: EguiContexts,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    texts: Query<(&Transform, &TextAnnotation), Without<EditingText>>,
    map_data: Res<MapData>,
) {
    if !is_annotation_layer_visible(&map_data) {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    for (transform, text) in texts.iter() {
        // Skip empty text
        if text.content.is_empty() {
            continue;
        }

        let world_pos = transform.translation.truncate();

        // Convert world position to screen position
        let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_pos.extend(0.0))
        else {
            continue;
        };

        // Convert Bevy color to egui color
        let srgba = text.color.to_srgba();
        let egui_color = egui::Color32::from_rgba_unmultiplied(
            (srgba.red * 255.0) as u8,
            (srgba.green * 255.0) as u8,
            (srgba.blue * 255.0) as u8,
            255,
        );

        // Render the text at the screen position using egui
        egui::Area::new(egui::Id::new(format!("text_annotation_{}", world_pos)))
            .fixed_pos(egui::pos2(screen_pos.x, screen_pos.y))
            .pivot(egui::Align2::LEFT_CENTER)
            .interactable(false)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(&text.content)
                        .color(egui_color)
                        .size(text.font_size),
                );
            });
    }
}

// ============================================================================
// Text Input UI
// ============================================================================

use bevy_egui::egui;

/// UI system for editing text annotations - shows an egui text input
pub fn text_annotation_input_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut text_state: ResMut<TextEditState>,
    mut editing_query: Query<(Entity, &Transform, &mut TextAnnotation), With<EditingText>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Only show if we're editing a text annotation
    let Some(editing_entity) = text_state.editing_entity else {
        return;
    };

    // Get the editing text annotation
    let Ok((entity, transform, mut text_annotation)) = editing_query.get_mut(editing_entity)
    else {
        // Entity no longer exists, clear editing state
        text_state.editing_entity = None;
        text_state.text_buffer.clear();
        return;
    };

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // Convert world position to screen position for the egui window
    let world_pos = transform.translation.truncate();

    // Get camera for coordinate conversion
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    // Convert world position to screen position
    let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_pos.extend(0.0)) else {
        return;
    };

    // Check for escape to cancel
    if keyboard.just_pressed(KeyCode::Escape) {
        // Cancel editing - remove if empty, otherwise keep original content
        if text_annotation.content.is_empty() && text_state.text_buffer.is_empty() {
            commands.entity(entity).despawn();
        } else {
            commands.entity(entity).remove::<EditingText>();
        }
        text_state.editing_entity = None;
        text_state.text_buffer.clear();
        return;
    }

    let mut should_finalize = false;

    // Create an egui Area at the text position
    egui::Area::new(egui::Id::new("text_annotation_input"))
        .fixed_pos(egui::pos2(screen_pos.x, screen_pos.y))
        .pivot(egui::Align2::LEFT_CENTER)
        .show(ctx, |ui| {
            ui.set_min_width(150.0);
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text_state.text_buffer)
                        .hint_text("Enter text...")
                        .desired_width(200.0)
                        .font(egui::TextStyle::Body),
                );

                // Auto-focus the text input
                if response.gained_focus() || text_state.text_buffer.is_empty() {
                    response.request_focus();
                }

                // Finalize on Enter
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    should_finalize = true;
                }

                // Show hint
                ui.label("Press Enter to confirm, Esc to cancel");
            });
        });

    if should_finalize {
        // Save the text
        text_annotation.content = text_state.text_buffer.clone();

        // If empty, delete the annotation
        if text_annotation.content.trim().is_empty() {
            commands.entity(entity).despawn();
        } else {
            commands.entity(entity).remove::<EditingText>();
        }

        text_state.editing_entity = None;
        text_state.text_buffer.clear();
    }
}
