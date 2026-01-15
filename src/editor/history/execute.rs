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

/// Execute a redo operation (same as executing the original command)
pub fn execute_redo(
    command: &EditorCommand,
    commands: &mut Commands,
    asset_server: &AssetServer,
    items_query: &Query<(Entity, &Transform, &PlacedItem)>,
    paths_query: &Query<(Entity, &DrawnPath), With<AnnotationMarker>>,
    lines_query: &Query<(Entity, &DrawnLine), With<AnnotationMarker>>,
    texts_query: &Query<(Entity, &Transform, &TextAnnotation), With<AnnotationMarker>>,
) -> Option<EditorCommand> {
    // Redo is the opposite of undo - execute the command forward
    match command {
        EditorCommand::PlaceItems { items } => {
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
        EditorCommand::DeleteItems { items } => {
            for item in items {
                commands.entity(item.entity).despawn();
            }
            Some(EditorCommand::DeleteItems {
                items: items.clone(),
            })
        }
        EditorCommand::MoveItems { transforms } => {
            let mut reverse_transforms = Vec::new();
            for (entity, old_transform, new_transform) in transforms {
                if let Ok((_, _, _)) = items_query.get(*entity) {
                    commands
                        .entity(*entity)
                        .insert(Transform::from(*new_transform));
                    reverse_transforms.push((*entity, *new_transform, *old_transform));
                }
            }
            Some(EditorCommand::MoveItems {
                transforms: reverse_transforms,
            })
        }
        EditorCommand::CreatePath { entity: _, path } => {
            let entity = spawn_path(commands, path);
            Some(EditorCommand::CreatePath {
                entity,
                path: path.clone(),
            })
        }
        EditorCommand::DeletePath { path } => {
            // Find and delete the matching path
            for (entity, existing_path) in paths_query.iter() {
                if existing_path.points == path.points {
                    commands.entity(entity).despawn();
                    break;
                }
            }
            Some(EditorCommand::DeletePath { path: path.clone() })
        }
        EditorCommand::CreateLine { entity: _, line } => {
            let entity = spawn_line(commands, line);
            Some(EditorCommand::CreateLine {
                entity,
                line: line.clone(),
            })
        }
        EditorCommand::DeleteLine { line } => {
            // Find and delete the matching line
            for (entity, existing_line) in lines_query.iter() {
                if existing_line.start == line.start && existing_line.end == line.end {
                    commands.entity(entity).despawn();
                    break;
                }
            }
            Some(EditorCommand::DeleteLine { line: line.clone() })
        }
        EditorCommand::CreateText { entity: _, text } => {
            let entity = spawn_text(commands, text);
            Some(EditorCommand::CreateText {
                entity,
                text: text.clone(),
            })
        }
        EditorCommand::DeleteText { text } => {
            // Find and delete the matching text
            for (entity, _, existing_text) in texts_query.iter() {
                if existing_text.content == text.text {
                    commands.entity(entity).despawn();
                    break;
                }
            }
            Some(EditorCommand::DeleteText { text: text.clone() })
        }
    }
}
