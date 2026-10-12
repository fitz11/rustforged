//! What the undo history remembers about things on the map.

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use crate::map::{ItemId, PlacedItem};

use super::super::annotations::{DrawnLine, DrawnPath, TextAnnotation};

/// A placed item or annotation as the undo history remembers it: enough to spawn it
/// again, or to put an existing one back the way it was. Found again by its [`ItemId`].
#[derive(Clone, Debug, PartialEq)]
pub enum MapObject {
    Item {
        id: ItemId,
        item: PlacedItem,
        transform: Transform,
    },
    Path {
        id: ItemId,
        path: DrawnPath,
    },
    Line {
        id: ItemId,
        line: DrawnLine,
    },
    Text {
        id: ItemId,
        text: TextAnnotation,
        position: Vec2,
    },
}

impl MapObject {
    /// A placed item, with `transform` moved to the item's depth (keeping its place in the
    /// stack of items on the same layer and z-index).
    pub fn item(id: ItemId, item: PlacedItem, transform: Transform) -> Self {
        let transform = item.at_depth(transform);
        Self::Item {
            id,
            item,
            transform,
        }
    }

    /// A placed item that isn't on the map yet.
    pub fn new_item(item: PlacedItem, transform: Transform) -> Self {
        Self::item(ItemId::new(), item, transform)
    }

    /// A freehand path that isn't on the map yet.
    pub fn new_path(path: DrawnPath) -> Self {
        Self::Path {
            id: ItemId::new(),
            path,
        }
    }

    /// A line that isn't on the map yet.
    pub fn new_line(line: DrawnLine) -> Self {
        Self::Line {
            id: ItemId::new(),
            line,
        }
    }

    /// A text annotation that isn't on the map yet.
    #[allow(dead_code)] // Text tool disabled - see TODO in tools.rs
    pub fn new_text(text: TextAnnotation, position: Vec2) -> Self {
        Self::Text {
            id: ItemId::new(),
            text,
            position,
        }
    }

    pub fn id(&self) -> ItemId {
        match self {
            Self::Item { id, .. }
            | Self::Path { id, .. }
            | Self::Line { id, .. }
            | Self::Text { id, .. } => *id,
        }
    }

    /// Spawns this object as a new entity that keeps its id.
    pub fn spawn(&self, commands: &mut Commands, asset_server: &AssetServer) -> Entity {
        let id = self.id();
        match self {
            Self::Item {
                item, transform, ..
            } => commands.spawn((item.clone().bundle(asset_server, *transform), id)),
            Self::Path { path, .. } => commands.spawn((path.clone().bundle(), id)),
            Self::Line { line, .. } => commands.spawn((line.clone().bundle(), id)),
            Self::Text { text, position, .. } => {
                commands.spawn((text.clone().bundle(*position), id))
            }
        }
        .id()
    }

    /// Puts an existing entity back into this state.
    pub fn apply(&self, entity: &mut EntityCommands) {
        match self {
            Self::Item {
                item, transform, ..
            } => {
                // Not the whole bundle: the sprite (and any missing-asset placeholder)
                // stays as it is
                entity.insert((*transform, item.clone(), item.layer.render_layers()));
            }
            Self::Path { path, .. } => {
                entity.insert(path.clone());
            }
            Self::Line { line, .. } => {
                entity.insert(line.clone());
            }
            Self::Text { text, position, .. } => {
                entity.insert(text.clone().bundle(*position));
            }
        }
    }

    /// The placed item's asset path, if this is a placed item.
    pub fn asset_path_mut(&mut self) -> Option<&mut String> {
        match self {
            Self::Item { item, .. } => Some(&mut item.asset_path),
            _ => None,
        }
    }
}
