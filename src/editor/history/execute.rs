//! Execute functions for undo and redo operations.

use bevy::prelude::*;

use crate::map::PlacedItem;

use super::super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use super::commands::EditorCommand;
use super::data_types::PlacedItemData;
use super::spawn_helpers::{spawn_line, spawn_path, spawn_placed_item, spawn_text};

/// Execute an undo operation and return the reverse command for redo
pub fn execute_undo(
    command: &EditorCommand,
    commands: &mut Commands,
    asset_server: &AssetServer,
    items_query: &Query<(Entity, &Transform, &PlacedItem)>,
    _paths_query: &Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    _lines_query: &Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    _texts_query: &Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) -> Option<EditorCommand> {
    match command {
        EditorCommand::PlaceItems { items } => {
            // Undo placement = delete items
            for item in items {
                commands.entity(item.entity).despawn();
            }
            Some(EditorCommand::DeleteItems {
                items: items.clone(),
            })
        }
        EditorCommand::DeleteItems { items } => {
            // Undo deletion = recreate items
            let mut new_items = Vec::new();
            for item in items {
                let entity = spawn_placed_item(commands, asset_server, item);
                new_items.push(PlacedItemData {
                    entity,
                    asset_path: item.asset_path.clone(),
                    layer: item.layer,
                    z_index: item.z_index,
                    transform: item.transform,
                });
            }
            Some(EditorCommand::PlaceItems { items: new_items })
        }
        EditorCommand::MoveItems { transforms } => {
            // Undo move = restore old transforms
            let mut reverse_transforms = Vec::new();
            for (entity, old_transform, new_transform) in transforms {
                if let Ok((_, _current_transform, _)) = items_query.get(*entity) {
                    commands
                        .entity(*entity)
                        .insert(Transform::from(*old_transform));
                    reverse_transforms.push((*entity, *new_transform, *old_transform));
                }
            }
            Some(EditorCommand::MoveItems {
                transforms: reverse_transforms,
            })
        }
        EditorCommand::CreatePath { entity, path } => {
            // Undo path creation = delete path
            commands.entity(*entity).despawn();
            Some(EditorCommand::DeletePath { path: path.clone() })
        }
        EditorCommand::DeletePath { path } => {
            // Undo path deletion = recreate path
            let entity = spawn_path(commands, path);
            Some(EditorCommand::CreatePath {
                entity,
                path: path.clone(),
            })
        }
        EditorCommand::CreateLine { entity, line } => {
            // Undo line creation = delete line
            commands.entity(*entity).despawn();
            Some(EditorCommand::DeleteLine { line: line.clone() })
        }
        EditorCommand::DeleteLine { line } => {
            // Undo line deletion = recreate line
            let entity = spawn_line(commands, line);
            Some(EditorCommand::CreateLine {
                entity,
                line: line.clone(),
            })
        }
        EditorCommand::CreateText { entity, text } => {
            // Undo text creation = delete text
            commands.entity(*entity).despawn();
            Some(EditorCommand::DeleteText { text: text.clone() })
        }
        EditorCommand::DeleteText { text } => {
            // Undo text deletion = recreate text
            let entity = spawn_text(commands, text);
            Some(EditorCommand::CreateText {
                entity,
                text: text.clone(),
            })
        }
    }
}

/// Execute a redo operation and return the reverse command for undo.
///
/// The redo stack stores the *inverse* commands produced by `execute_undo`, so
/// redoing is the same inversion operation as undoing: inverting a stored
/// inverse re-applies the original action and yields the original command back
/// for the undo stack. Delegating to `execute_undo` keeps the two perfectly
/// symmetric and avoids the fragile content-based entity matching that a
/// separate forward implementation required.
pub fn execute_redo(
    command: &EditorCommand,
    commands: &mut Commands,
    asset_server: &AssetServer,
    items_query: &Query<(Entity, &Transform, &PlacedItem)>,
    paths_query: &Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    lines_query: &Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    texts_query: &Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) -> Option<EditorCommand> {
    execute_undo(
        command,
        commands,
        asset_server,
        items_query,
        paths_query,
        lines_query,
        texts_query,
    )
}
