//! Bevy systems for handling undo/redo keyboard shortcuts.

use bevy::prelude::*;

use crate::input::{command_held, shift_held};
use crate::map::{ItemId, MapDirtyState};

use super::command_history::CommandHistory;
use super::execute::execute;

/// Ctrl+Z undoes; Ctrl+Y or Ctrl+Shift+Z redoes (Cmd on macOS).
pub fn handle_undo_redo(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut history: ResMut<CommandHistory>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ids: Query<(Entity, &ItemId)>,
    mut dirty_state: ResMut<MapDirtyState>,
) {
    if !command_held(&keyboard) {
        return;
    }
    let z = keyboard.just_pressed(KeyCode::KeyZ);
    let undo = z && !shift_held(&keyboard);
    let redo = keyboard.just_pressed(KeyCode::KeyY) || (z && shift_held(&keyboard));
    if !undo && !redo {
        return;
    }

    let command = if undo {
        history.pop_undo()
    } else {
        history.pop_redo()
    };
    let Some(command) = command else {
        return;
    };

    let entities = ids.iter().map(|(entity, id)| (*id, entity)).collect();
    let inverse = execute(&command, &mut commands, &asset_server, &entities);
    if undo {
        history.push_redo(inverse);
    } else {
        history.push_undo(inverse);
    }
    dirty_state.is_dirty = true;
}
