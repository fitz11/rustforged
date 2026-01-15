//! Bevy systems for handling undo/redo keyboard shortcuts.

use bevy::prelude::*;

use crate::map::PlacedItem;

use super::super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use super::command_history::CommandHistory;
use super::execute::{execute_redo, execute_undo};

/// System to handle undo keyboard shortcut (Ctrl+Z)
#[allow(clippy::too_many_arguments)]
pub fn handle_undo(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut history: ResMut<CommandHistory>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    items_query: Query<(Entity, &Transform, &PlacedItem)>,
    paths_query: Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    lines_query: Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    texts_query: Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    // Ctrl+Z (without shift) = undo
    if ctrl
        && !shift
        && keyboard.just_pressed(KeyCode::KeyZ)
        && let Some(command) = history.pop_undo()
    {
        let reverse_command = execute_undo(
            &command,
            &mut commands,
            &asset_server,
            &items_query,
            &paths_query,
            &lines_query,
            &texts_query,
        );
        if let Some(reverse) = reverse_command {
            history.push_redo(reverse);
        }
    }
}

/// System to handle redo keyboard shortcut (Ctrl+Y or Ctrl+Shift+Z)
#[allow(clippy::too_many_arguments)]
pub fn handle_redo(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut history: ResMut<CommandHistory>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    items_query: Query<(Entity, &Transform, &PlacedItem)>,
    paths_query: Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    lines_query: Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    texts_query: Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    // Ctrl+Y or Ctrl+Shift+Z = redo
    let redo_pressed = (ctrl && keyboard.just_pressed(KeyCode::KeyY))
        || (ctrl && shift && keyboard.just_pressed(KeyCode::KeyZ));

    if redo_pressed
        && let Some(command) = history.pop_redo()
    {
        let reverse_command = execute_redo(
            &command,
            &mut commands,
            &asset_server,
            &items_query,
            &paths_query,
            &lines_query,
            &texts_query,
        );
        if let Some(reverse) = reverse_command {
            history.push_undo(reverse);
        }
    }
}
