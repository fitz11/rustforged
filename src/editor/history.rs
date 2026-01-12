//! Undo/Redo system for editor actions.
//!
//! This module provides a command history that allows users to undo and redo their actions.
//! All reversible editor operations (placement, deletion, movement, etc.) are recorded as
//! commands that can be undone and redone.
//!
//! ## Usage
//!
//! - **Ctrl+Z**: Undo the last action
//! - **Ctrl+Y** or **Ctrl+Shift+Z**: Redo the last undone action
//!
//! ## Supported Operations
//!
//! - Item placement and deletion
//! - Item movement (transform changes)
//! - Annotation creation and deletion (paths, lines, text)

use bevy::prelude::*;

use crate::map::{Layer, PlacedItem};

use super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};

/// Maximum number of commands to keep in history
#[allow(dead_code)]
const MAX_HISTORY_SIZE: usize = 100;

/// A reversible command in the editor
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum EditorCommand {
    /// Item(s) were placed on the map
    PlaceItems {
        items: Vec<PlacedItemData>,
    },
    /// Item(s) were deleted from the map
    DeleteItems {
        items: Vec<PlacedItemData>,
    },
    /// Item(s) were moved/transformed
    MoveItems {
        /// Entity ID, old transform, new transform
        transforms: Vec<(Entity, TransformData, TransformData)>,
    },
    /// A freehand path was created
    CreatePath {
        entity: Entity,
        path: PathData,
    },
    /// A freehand path was deleted
    DeletePath {
        path: PathData,
    },
    /// A line was created
    CreateLine {
        entity: Entity,
        line: LineData,
    },
    /// A line was deleted
    DeleteLine {
        line: LineData,
    },
    /// A text annotation was created
    CreateText {
        entity: Entity,
        text: TextData,
    },
    /// A text annotation was deleted
    DeleteText {
        text: TextData,
    },
}

/// Serializable data for a placed item
#[derive(Clone, Debug)]
pub struct PlacedItemData {
    pub entity: Entity,
    pub asset_path: String,
    pub layer: Layer,
    pub z_index: i32,
    pub transform: TransformData,
}

/// Serializable transform data
#[derive(Clone, Debug, Copy)]
pub struct TransformData {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl From<&Transform> for TransformData {
    fn from(t: &Transform) -> Self {
        Self {
            translation: t.translation,
            rotation: t.rotation,
            scale: t.scale,
        }
    }
}

impl From<TransformData> for Transform {
    fn from(t: TransformData) -> Self {
        Transform {
            translation: t.translation,
            rotation: t.rotation,
            scale: t.scale,
        }
    }
}

/// Serializable data for a drawn path
#[derive(Clone, Debug)]
pub struct PathData {
    pub points: Vec<Vec2>,
    pub color: Color,
    pub stroke_width: f32,
}

impl From<&DrawnPath> for PathData {
    fn from(p: &DrawnPath) -> Self {
        Self {
            points: p.points.clone(),
            color: p.color,
            stroke_width: p.stroke_width,
        }
    }
}

/// Serializable data for a drawn line
#[derive(Clone, Debug)]
pub struct LineData {
    pub start: Vec2,
    pub end: Vec2,
    pub color: Color,
    pub stroke_width: f32,
}

impl From<&DrawnLine> for LineData {
    fn from(l: &DrawnLine) -> Self {
        Self {
            start: l.start,
            end: l.end,
            color: l.color,
            stroke_width: l.stroke_width,
        }
    }
}

/// Serializable data for a text annotation
#[derive(Clone, Debug)]
pub struct TextData {
    pub text: String,
    pub position: Vec2,
    pub color: Color,
    pub font_size: f32,
}

/// Resource tracking command history for undo/redo
#[derive(Resource, Default)]
pub struct CommandHistory {
    /// Stack of commands that can be undone (most recent last)
    undo_stack: Vec<EditorCommand>,
    /// Stack of commands that can be redone (most recent last)
    redo_stack: Vec<EditorCommand>,
}

#[allow(dead_code)]
impl CommandHistory {
    /// Push a new command to the history
    pub fn push(&mut self, command: EditorCommand) {
        // Clear redo stack when a new action is performed
        self.redo_stack.clear();

        self.undo_stack.push(command);

        // Trim history if it exceeds max size
        while self.undo_stack.len() > MAX_HISTORY_SIZE {
            self.undo_stack.remove(0);
        }
    }

    /// Pop the last command for undo
    pub fn pop_undo(&mut self) -> Option<EditorCommand> {
        self.undo_stack.pop()
    }

    /// Pop the last command for redo
    pub fn pop_redo(&mut self) -> Option<EditorCommand> {
        self.redo_stack.pop()
    }

    /// Push a command to the redo stack (used after undo)
    pub fn push_redo(&mut self, command: EditorCommand) {
        self.redo_stack.push(command);
    }

    /// Push a command to the undo stack (used after redo)
    pub fn push_undo(&mut self, command: EditorCommand) {
        self.undo_stack.push(command);
    }

    /// Check if there are commands to undo
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if there are commands to redo
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the count of undoable commands
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the count of redoable commands
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

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
    let redo_pressed =
        (ctrl && keyboard.just_pressed(KeyCode::KeyY)) || (ctrl && shift && keyboard.just_pressed(KeyCode::KeyZ));

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

/// Execute an undo operation and return the reverse command for redo
fn execute_undo(
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
                    commands.entity(*entity).insert(Transform::from(*old_transform));
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
fn execute_redo(
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
                    commands.entity(*entity).insert(Transform::from(*new_transform));
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

/// Helper to spawn a placed item from PlacedItemData
fn spawn_placed_item(commands: &mut Commands, asset_server: &AssetServer, data: &PlacedItemData) -> Entity {
    use bevy::camera::visibility::RenderLayers;

    let texture_handle: Handle<Image> = asset_server.load(&data.asset_path);

    commands
        .spawn((
            Sprite::from_image(texture_handle),
            Transform::from(data.transform),
            PlacedItem {
                asset_path: data.asset_path.clone(),
                layer: data.layer,
                z_index: data.z_index,
            },
            RenderLayers::from_layers(&[0, 1]),
        ))
        .id()
}

/// Helper to spawn a drawn path from PathData
fn spawn_path(commands: &mut Commands, data: &PathData) -> Entity {
    use super::annotations::AnnotationMarker;
    use crate::map::Layer;

    commands
        .spawn((
            DrawnPath {
                points: data.points.clone(),
                color: data.color,
                stroke_width: data.stroke_width,
            },
            Transform::from_xyz(0.0, 0.0, Layer::Annotation.z_base()),
            AnnotationMarker,
        ))
        .id()
}

/// Helper to spawn a drawn line from LineData
fn spawn_line(commands: &mut Commands, data: &LineData) -> Entity {
    use super::annotations::AnnotationMarker;
    use crate::map::Layer;

    commands
        .spawn((
            DrawnLine {
                start: data.start,
                end: data.end,
                color: data.color,
                stroke_width: data.stroke_width,
            },
            Transform::from_xyz(0.0, 0.0, Layer::Annotation.z_base()),
            AnnotationMarker,
        ))
        .id()
}

/// Helper to spawn a text annotation from TextData
fn spawn_text(commands: &mut Commands, data: &TextData) -> Entity {
    use super::annotations::AnnotationMarker;
    use crate::map::Layer;

    commands
        .spawn((
            TextAnnotation {
                content: data.text.clone(),
                color: data.color,
                font_size: data.font_size,
            },
            Transform::from_xyz(data.position.x, data.position.y, Layer::Annotation.z_base()),
            AnnotationMarker,
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
