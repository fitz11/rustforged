//! Text tool system for creating and editing text annotations.
//!
//! Note: Text tool is currently disabled - see TODO in tools.rs

use bevy::prelude::*;
use bevy_egui::egui;
use bevy_egui::EguiContexts;

use crate::map::{Layer, MapData};

use super::super::camera::EditorCamera;
use super::super::params::{is_cursor_over_ui, CameraParams};
use super::super::tools::{CurrentTool, EditorTool};
use super::components::{AnnotationMarker, EditingText, TextAnnotation};
use super::layer_helpers::is_annotation_layer_locked;
use super::state::{AnnotationSettings, TextEditState};

// Text tool disabled - see TODO in tools.rs
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn handle_text(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    current_tool: Res<CurrentTool>,
    mut text_state: ResMut<TextEditState>,
    settings: Res<AnnotationSettings>,
    map_data: Res<MapData>,
    camera: CameraParams,
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

    if is_cursor_over_ui(&mut contexts) {
        return;
    }

    let Some(world_pos) = camera.cursor_world_pos() else {
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

// Text tool disabled - see TODO in tools.rs
#[allow(dead_code)]
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
    let Ok((entity, transform, mut text_annotation)) = editing_query.get_mut(editing_entity) else {
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

                // Request focus only when not already focused
                if !response.has_focus() {
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
