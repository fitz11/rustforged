//! Unit tests for the history module.

use bevy::prelude::*;

use super::command_history::CommandHistory;
use super::commands::EditorCommand;
use super::data_types::TransformData;
use super::MAX_HISTORY_SIZE;

#[test]
fn test_command_history_push() {
    let mut history = CommandHistory::default();
    assert!(!history.can_undo());

    history.push(EditorCommand::PlaceItems { items: vec![] });
    assert!(history.can_undo());
    assert_eq!(history.undo_count(), 1);
}

#[test]
fn test_command_history_undo_clears_redo() {
    let mut history = CommandHistory::default();

    // Push some commands
    history.push(EditorCommand::PlaceItems { items: vec![] });
    history.push(EditorCommand::PlaceItems { items: vec![] });

    // Pop one for undo
    history.pop_undo();
    history.push_redo(EditorCommand::PlaceItems { items: vec![] });
    assert!(history.can_redo());

    // Push a new command - should clear redo
    history.push(EditorCommand::PlaceItems { items: vec![] });
    assert!(!history.can_redo());
}

#[test]
fn test_command_history_max_size() {
    let mut history = CommandHistory::default();

    // Push more than max size
    for _ in 0..150 {
        history.push(EditorCommand::PlaceItems { items: vec![] });
    }

    // Should be trimmed to max size
    assert_eq!(history.undo_count(), MAX_HISTORY_SIZE);
}

#[test]
fn test_transform_data_conversion() {
    let transform = Transform {
        translation: Vec3::new(1.0, 2.0, 3.0),
        rotation: Quat::from_rotation_z(0.5),
        scale: Vec3::new(2.0, 2.0, 1.0),
    };

    let data = TransformData::from(&transform);
    let restored: Transform = data.into();

    assert_eq!(transform.translation, restored.translation);
    assert_eq!(transform.rotation, restored.rotation);
    assert_eq!(transform.scale, restored.scale);
}
