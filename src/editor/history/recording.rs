//! Central command recording for undo/redo.
//!
//! Editing systems emit [`RecordEditorCommand`] when they complete a reversible
//! action. A single system ([`record_commands`]) applies these to the
//! [`CommandHistory`], so individual editing systems don't each need mutable
//! access to the history resource (which would create scheduling conflicts).

use bevy::prelude::*;

use super::command_history::CommandHistory;
use super::commands::EditorCommand;
use super::data_types::MapObject;

/// Emitted by editing systems to record a completed, reversible action onto the
/// undo history. Recording a new command clears the redo stack.
#[derive(Message)]
pub struct RecordEditorCommand {
    pub command: EditorCommand,
}

/// Takes `selection` off the map as one undo step (Delete and Cut).
pub fn remove_from_map(
    commands: &mut Commands,
    history_writer: &mut MessageWriter<RecordEditorCommand>,
    selection: Vec<(Entity, MapObject)>,
) {
    if selection.is_empty() {
        return;
    }
    let (entities, objects): (Vec<_>, Vec<_>) = selection.into_iter().unzip();
    history_writer.write(RecordEditorCommand {
        command: EditorCommand::Remove(objects),
    });
    for entity in entities {
        commands.entity(entity).despawn();
    }
}

/// Applies recorded commands to the [`CommandHistory`].
pub fn record_commands(
    mut history: ResMut<CommandHistory>,
    mut events: MessageReader<RecordEditorCommand>,
) {
    for event in events.read() {
        history.push(event.command.clone());
    }
}
