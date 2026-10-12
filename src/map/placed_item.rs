use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::assets::library_asset_path;

use super::Layer;

/// Identifies a placed item or annotation for the undo history. Undoing a delete spawns
/// a new entity, so the history can't hold on to entities; it finds things by this id.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(u64);

impl ItemId {
    /// A fresh id, different from every other in this run of the app.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// Items on the same layer and z-index stack in the order they were added: each sits this
/// much above the one before it. Without it they'd share a depth, and the renderer and
/// click testing would order them arbitrarily (selecting one could change which is on top).
pub const STACK_STEP: f32 = 0.001;
/// Highest stacking offset; items added past it share the top
pub const MAX_STACK_OFFSET: f32 = 0.99;

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[require(ItemId = ItemId::new())]
pub struct PlacedItem {
    pub asset_path: String,
    pub layer: Layer,
    pub z_index: i32,
}

impl PlacedItem {
    /// Components for a placed item entity: its sprite, `transform` moved to the item's
    /// depth, and render layers that keep GM-only items out of the player view. Every
    /// place that spawns an item uses this, so they can't disagree about what players see.
    pub fn bundle(self, asset_server: &AssetServer, transform: Transform) -> impl Bundle {
        let z_index = self.z_index.clamp(0, Layer::max_z_index());
        let item = PlacedItem { z_index, ..self };
        let transform = item.at_depth(transform);
        let texture: Handle<Image> = asset_server.load(library_asset_path(&item.asset_path));
        let render_layers = item.layer.render_layers();
        (Sprite::from_image(texture), transform, item, render_layers)
    }

    /// The item's depth: its layer's base plus its z-index within the layer.
    pub fn depth(&self) -> f32 {
        self.layer.z_base() + self.z_index as f32
    }

    /// Where an item at depth `z` sits in the stack of items on its layer and z-index.
    pub fn stack_offset(&self, z: f32) -> f32 {
        (z - self.depth()).clamp(0.0, MAX_STACK_OFFSET)
    }

    /// `transform` moved to this item's depth, keeping its place in the stack.
    pub fn at_depth(&self, mut transform: Transform) -> Transform {
        transform.translation.z = self.depth() + self.stack_offset(transform.translation.z);
        transform
    }

    fn stack_key(&self) -> (Layer, i32) {
        (self.layer, self.z_index)
    }
}

/// Puts new items on top of the stack of items on their layer and z-index.
#[derive(SystemParam)]
pub struct StackTops<'w, 's, F: QueryFilter + 'static = ()> {
    items: Query<'w, 's, (&'static PlacedItem, &'static Transform), F>,
}

impl<F: QueryFilter> StackTops<'_, '_, F> {
    /// Depth for one new `item`: just above the others on its layer and z-index.
    pub fn z_on_top(&self, item: &PlacedItem) -> f32 {
        self.stacker().next_z(item)
    }

    /// For adding several items at once, each above the one before.
    pub fn stacker(&self) -> Stacker {
        let mut next = HashMap::new();
        for (item, transform) in &self.items {
            let above =
                (item.stack_offset(transform.translation.z) + STACK_STEP).min(MAX_STACK_OFFSET);
            let entry = next.entry(item.stack_key()).or_insert(above);
            *entry = entry.max(above);
        }
        Stacker { next }
    }
}

/// Hands out depths for new items, each on top of the ones before it on its layer and
/// z-index. Also stacks a loaded map's items in the order they're listed in the file.
#[derive(Default)]
pub struct Stacker {
    next: HashMap<(Layer, i32), f32>,
}

impl Stacker {
    pub fn next_z(&mut self, item: &PlacedItem) -> f32 {
        let offset = self.next.entry(item.stack_key()).or_insert(0.0);
        let z = item.depth() + *offset;
        *offset = (*offset + STACK_STEP).min(MAX_STACK_OFFSET);
        z
    }
}

#[derive(Component)]
pub struct Selected;

/// Marker component for items whose asset file is missing or failed to load.
/// The original_path stores the path that was requested but couldn't be found.
#[derive(Component)]
pub struct MissingAsset {
    /// The original asset path that was requested but couldn't be found.
    /// Used for displaying error information in the UI.
    #[allow(dead_code)]
    pub original_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn token(z_index: i32) -> PlacedItem {
        PlacedItem {
            asset_path: "token.png".to_string(),
            layer: Layer::Token,
            z_index,
        }
    }

    #[test]
    fn test_stacker_puts_each_item_above_the_last_in_its_group() {
        let mut stacker = Stacker::default();
        let first = stacker.next_z(&token(0));
        let second = stacker.next_z(&token(0));
        let other_slot = stacker.next_z(&token(1));
        assert!(second > first);
        assert_eq!(first, token(0).depth());
        assert_eq!(other_slot, token(1).depth());
        // Still below the next z-index
        assert!(second < token(1).depth());
    }

    #[test]
    fn test_at_depth_keeps_the_stack_offset() {
        let item = token(2);
        let transform = Transform::from_xyz(0.0, 0.0, item.depth() + 0.005);
        assert_eq!(item.at_depth(transform).translation.z, item.depth() + 0.005);
        // A transform from another layer's depth lands at the bottom of this one
        let elsewhere = Transform::from_xyz(0.0, 0.0, 3.0);
        assert_eq!(item.at_depth(elsewhere).translation.z, item.depth());
    }

    #[test]
    fn test_stack_tops_go_above_existing_items() {
        let mut world = World::new();
        world.spawn((
            token(0),
            Transform::from_xyz(0.0, 0.0, token(0).depth() + 0.002),
        ));
        let z = world
            .run_system_once(|stack: StackTops| stack.z_on_top(&token(0)))
            .unwrap();
        assert!((z - (token(0).depth() + 0.003)).abs() < 1e-4);
    }
}
