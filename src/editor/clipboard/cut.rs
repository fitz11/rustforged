//! Cut system for clipboard operations (Ctrl+X).

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map::{PlacedItem, SavedLine, SavedPath, SavedPlacedItem, SavedTextBox, Selected};

use super::super::params::SelectedAnnotationQueries;
use super::helpers::{color_to_array, path_center};
use super::types::{Clipboard, ClipboardLine, ClipboardPath, ClipboardPlacedItem, ClipboardText};

/// Cut selected items to clipboard (Ctrl+X) - copy then delete
pub fn handle_cut(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut clipboard: ResMut<Clipboard>,
    mut contexts: EguiContexts,
    selected_items: Query<(Entity, &PlacedItem, &Transform), With<Selected>>,
    annotations: SelectedAnnotationQueries,
) {
    // Check for Ctrl+X
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    if !ctrl || !keyboard.just_pressed(KeyCode::KeyX) {
        return;
    }

    // Don't cut if UI has keyboard focus
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    // Nothing selected? Do nothing
    if selected_items.is_empty()
        && annotations.paths.is_empty()
        && annotations.lines.is_empty()
        && annotations.texts.is_empty()
    {
        return;
    }

    // Calculate centroid - need separate queries for the calculation
    let mut positions: Vec<Vec2> = Vec::new();
    for (_, _, transform) in selected_items.iter() {
        positions.push(transform.translation.truncate());
    }
    for (_, path) in annotations.paths.iter() {
        positions.push(path_center(path));
    }
    for (_, line) in annotations.lines.iter() {
        positions.push((line.start + line.end) / 2.0);
    }
    for (_, transform, _) in annotations.texts.iter() {
        positions.push(transform.translation.truncate());
    }

    let centroid = if positions.is_empty() {
        Vec2::ZERO
    } else {
        let sum: Vec2 = positions.iter().copied().sum();
        sum / positions.len() as f32
    };

    // Clear clipboard
    clipboard.clear();

    // Copy and delete placed items
    for (entity, item, transform) in selected_items.iter() {
        let saved = SavedPlacedItem::from_entity(item, transform);
        let offset = saved.position - centroid;
        clipboard
            .placed_items
            .push(ClipboardPlacedItem { saved, offset });
        commands.entity(entity).despawn();
    }

    // Copy and delete paths
    for (entity, path) in annotations.paths.iter() {
        let center = path_center(path);
        let offset = center - centroid;
        let saved = SavedPath {
            points: path.points.clone(),
            color: color_to_array(path.color),
            stroke_width: path.stroke_width,
        };
        clipboard.paths.push(ClipboardPath { saved, offset });
        commands.entity(entity).despawn();
    }

    // Copy and delete lines
    for (entity, line) in annotations.lines.iter() {
        let line_center = (line.start + line.end) / 2.0;
        let offset = line_center - centroid;
        let saved = SavedLine {
            start: line.start,
            end: line.end,
            color: color_to_array(line.color),
            stroke_width: line.stroke_width,
        };
        clipboard.lines.push(ClipboardLine { saved, offset });
        commands.entity(entity).despawn();
    }

    // Copy and delete text annotations
    for (entity, transform, text) in annotations.texts.iter() {
        let pos = transform.translation.truncate();
        let offset = pos - centroid;
        let saved = SavedTextBox {
            position: pos,
            content: text.content.clone(),
            font_size: text.font_size,
            color: color_to_array(text.color),
        };
        clipboard.texts.push(ClipboardText { saved, offset });
        commands.entity(entity).despawn();
    }
}
