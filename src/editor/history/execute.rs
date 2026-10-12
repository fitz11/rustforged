//! Carrying out undo and redo.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::map::ItemId;

use super::commands::EditorCommand;

/// Carries out `command` and returns the command that reverses it.
///
/// Undo and redo both use this: the history keeps each step's inverse on the other stack,
/// so undoing an `Add` runs its inverse `Remove`, and redoing runs the `Add` again. Objects
/// are found by [`ItemId`], so a step still works after an earlier undo respawned them.
pub fn execute(
    command: &EditorCommand,
    commands: &mut Commands,
    asset_server: &AssetServer,
    entities: &HashMap<ItemId, Entity>,
) -> EditorCommand {
    match command {
        EditorCommand::Add(objects) => {
            for object in objects {
                if let Some(entity) = entities.get(&object.id()) {
                    commands.entity(*entity).despawn();
                }
            }
            EditorCommand::Remove(objects.clone())
        }
        EditorCommand::Remove(objects) => {
            for object in objects {
                object.spawn(commands, asset_server);
            }
            EditorCommand::Add(objects.clone())
        }
        EditorCommand::Change(pairs) => {
            for (before, _) in pairs {
                if let Some(entity) = entities.get(&before.id()) {
                    before.apply(&mut commands.entity(*entity));
                }
            }
            EditorCommand::Change(
                pairs
                    .iter()
                    .map(|(before, after)| (after.clone(), before.clone()))
                    .collect(),
            )
        }
    }
}
