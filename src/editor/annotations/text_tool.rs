//! Text tool: click to type a note on the map, click a note to edit it.
//!
//! Notes are annotations, so only the GM sees them. While one is being edited its text is
//! typed in a small box at its position; Enter (or clicking elsewhere, or switching tools)
//! keeps it, Esc throws the edit away. Each finished edit is one undo step.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::{FontSize, TextColor, TextFont};
use bevy_egui::{EguiContexts, egui};

use crate::map::{ItemId, MapData, render_layer};

use super::super::camera::EditorCamera;
use super::super::history::{EditorCommand, MapObject, RecordEditorCommand};
use super::super::params::{CameraParams, is_cursor_over_ui};
use super::components::{EditingText, TextAnnotation};
use super::hit_testing::point_in_text;
use super::layer_helpers::{is_annotation_layer_locked, is_annotation_layer_visible};
use super::state::{AnnotationSettings, TextEditState};

type Texts<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ItemId,
        &'static mut TextAnnotation,
        &'static Transform,
    ),
>;

/// Text tool clicks: edit the note under the cursor, or start a new one there.
#[allow(clippy::too_many_arguments)]
pub fn handle_text(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut text_state: ResMut<TextEditState>,
    settings: Res<AnnotationSettings>,
    map_data: Res<MapData>,
    camera: CameraParams,
    mut contexts: EguiContexts,
    mut texts: Texts,
    mut history: MessageWriter<RecordEditorCommand>,
) {
    if !mouse_button.just_pressed(MouseButton::Left)
        || is_annotation_layer_locked(&map_data)
        || is_cursor_over_ui(&mut contexts)
    {
        return;
    }
    let Some(world_pos) = camera.cursor_world_pos() else {
        return;
    };

    // A click away from the note being edited keeps it
    end_text_edit(
        true,
        &mut text_state,
        &mut commands,
        &mut texts,
        &mut history,
    );

    let clicked = texts
        .iter()
        .find(|(_, _, text, transform)| point_in_text(world_pos, transform, text))
        .map(|(entity, id, text, transform)| {
            (
                entity,
                MapObject::Text {
                    id: *id,
                    text: text.clone(),
                    position: transform.translation.truncate(),
                },
            )
        });

    if let Some((entity, before)) = clicked {
        if let MapObject::Text { text, .. } = &before {
            text_state.text_buffer = text.content.clone();
        }
        text_state.editing_entity = Some(entity);
        text_state.before = Some(before);
        commands.entity(entity).insert(EditingText);
    } else {
        let entity = commands
            .spawn((
                TextAnnotation {
                    content: String::new(),
                    font_size: settings.font_size,
                    color: settings.stroke_color,
                }
                .bundle(world_pos),
                EditingText,
            ))
            .id();
        text_state.editing_entity = Some(entity);
        text_state.before = None;
        text_state.text_buffer.clear();
    }
}

/// Ends the edit in progress: `keep` stores the typed text (an empty note is removed),
/// otherwise the note goes back to how it was (a new one is removed). Records the result
/// as one undo step.
fn end_text_edit(
    keep: bool,
    state: &mut TextEditState,
    commands: &mut Commands,
    texts: &mut Texts,
    history: &mut MessageWriter<RecordEditorCommand>,
) {
    let Some(entity) = state.editing_entity.take() else {
        return;
    };
    let before = state.before.take();
    let typed = std::mem::take(&mut state.text_buffer);
    let Ok((_, id, mut text, transform)) = texts.get_mut(entity) else {
        return;
    };
    let id = *id;
    let position = transform.translation.truncate();
    commands.entity(entity).remove::<EditingText>();

    if !keep {
        match &before {
            None => commands.entity(entity).despawn(),
            Some(MapObject::Text { text: original, .. }) => *text = original.clone(),
            Some(_) => {}
        }
        return;
    }

    let typed = typed.trim().to_string();
    if typed.is_empty() {
        commands.entity(entity).despawn();
        if let Some(before) = before {
            history.write(RecordEditorCommand {
                command: EditorCommand::Remove(vec![before]),
            });
        }
        return;
    }

    text.content = typed;
    let after = MapObject::Text {
        id,
        text: text.clone(),
        position,
    };
    let command = match before {
        None => Some(EditorCommand::Add(vec![after])),
        Some(before) => EditorCommand::change([(before, after)]),
    };
    if let Some(command) = command {
        history.write(RecordEditorCommand { command });
    }
}

/// The typing box for the note being edited, at its position on screen. Also finishes an
/// edit another system asked to end (see [`TextEditState::finish`]).
pub fn text_annotation_input_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut text_state: ResMut<TextEditState>,
    mut texts: Texts,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    mut history: MessageWriter<RecordEditorCommand>,
) -> Result {
    if let Some(keep) = text_state.finish.take() {
        end_text_edit(
            keep,
            &mut text_state,
            &mut commands,
            &mut texts,
            &mut history,
        );
    }
    let Some(entity) = text_state.editing_entity else {
        return Ok(());
    };
    let Ok((_, _, _, transform)) = texts.get(entity) else {
        // Deleted while being edited
        text_state.editing_entity = None;
        return Ok(());
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return Ok(());
    };
    let Ok(screen_pos) = camera.world_to_viewport(camera_transform, transform.translation) else {
        return Ok(());
    };

    let ctx = contexts.ctx_mut()?;
    let mut finish = None;
    // Foreground, like a popup: it isn't a dialog, and the map stays usable around it
    egui::Area::new(egui::Id::new("text_annotation_input"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(screen_pos.x, screen_pos.y))
        .pivot(egui::Align2::CENTER_CENTER)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text_state.text_buffer)
                        .hint_text("Type a note...")
                        .desired_width(200.0),
                );
                if !response.has_focus() && !response.lost_focus() {
                    response.request_focus();
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    finish = Some(true);
                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    finish = Some(false);
                }
                ui.label(
                    egui::RichText::new("Enter to keep, Esc to cancel")
                        .small()
                        .weak(),
                );
            });
        });

    if let Some(keep) = finish {
        end_text_edit(
            keep,
            &mut text_state,
            &mut commands,
            &mut texts,
            &mut history,
        );
    }
    Ok(())
}

/// Draws notes as text in the map, editor-only. Keeps each note's text in step with its
/// annotation, and hides it while it's being typed in (the typing box shows it) or while
/// the Annotations layer is hidden.
#[allow(clippy::type_complexity)]
pub fn sync_text_annotations(
    mut commands: Commands,
    changed: Query<(Entity, &TextAnnotation), Changed<TextAnnotation>>,
    mut texts: Query<(&mut Visibility, Has<EditingText>), With<TextAnnotation>>,
    map_data: Res<MapData>,
) {
    for (entity, text) in &changed {
        commands.entity(entity).insert((
            Text2d::new(text.content.clone()),
            TextFont {
                font_size: FontSize::Px(text.font_size),
                ..default()
            },
            TextColor(text.color),
            Anchor::CENTER,
            RenderLayers::layer(render_layer::EDITOR),
        ));
    }

    let layer_visible = is_annotation_layer_visible(&map_data);
    for (mut visibility, editing) in &mut texts {
        let shown = if layer_visible && !editing {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        visibility.set_if_neq(shown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::history::{CommandHistory, record_commands};
    use bevy::ecs::system::RunSystemOnce;

    fn app_with_note(content: &str) -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<TextEditState>()
            .init_resource::<CommandHistory>()
            .add_message::<RecordEditorCommand>()
            .add_systems(Update, record_commands);
        let entity = app
            .world_mut()
            .spawn((
                TextAnnotation {
                    content: content.to_string(),
                    font_size: 24.0,
                    color: Color::WHITE,
                }
                .bundle(Vec2::ZERO),
                EditingText,
            ))
            .id();
        app.world_mut()
            .resource_mut::<TextEditState>()
            .editing_entity = Some(entity);
        (app, entity)
    }

    fn end(app: &mut App, keep: bool, typed: &str) {
        app.world_mut().resource_mut::<TextEditState>().text_buffer = typed.to_string();
        app.world_mut()
            .run_system_once(
                move |mut state: ResMut<TextEditState>,
                      mut commands: Commands,
                      mut texts: Texts,
                      mut history: MessageWriter<RecordEditorCommand>| {
                    end_text_edit(keep, &mut state, &mut commands, &mut texts, &mut history);
                },
            )
            .unwrap();
        app.update();
    }

    #[test]
    fn test_new_note_is_kept_as_one_undo_step() {
        let (mut app, entity) = app_with_note("");
        end(&mut app, true, "  Trap  ");

        let text = app.world().get::<TextAnnotation>(entity).unwrap();
        assert_eq!(text.content, "Trap");
        assert!(!app.world().entity(entity).contains::<EditingText>());
        let mut history = app.world_mut().resource_mut::<CommandHistory>();
        assert!(matches!(history.pop_undo(), Some(EditorCommand::Add(_))));
    }

    #[test]
    fn test_cancelled_or_empty_new_note_is_removed() {
        let (mut app, entity) = app_with_note("");
        end(&mut app, false, "Trap");
        assert!(app.world().get_entity(entity).is_err());

        let (mut app, entity) = app_with_note("");
        end(&mut app, true, "   ");
        assert!(app.world().get_entity(entity).is_err());
        assert!(!app.world().resource::<CommandHistory>().can_undo());
    }
}
