//! Copy system for clipboard operations (Ctrl+C).

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map::{PlacedItem, SavedLine, SavedPath, SavedPlacedItem, SavedTextBox, Selected};

use super::super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use super::helpers::{calculate_selection_centroid, color_to_array, path_center};
use super::types::{Clipboard, ClipboardLine, ClipboardPath, ClipboardPlacedItem, ClipboardText};

/// Copy selected items to clipboard (Ctrl+C)
#[allow(clippy::type_complexity)]
pub fn handle_copy(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut clipboard: ResMut<Clipboard>,
    mut contexts: EguiContexts,
    // PlacedItem queries
    selected_items: Query<(&PlacedItem, &Transform), With<Selected>>,
    // Annotation queries
    selected_paths: Query<&DrawnPath, (With<Selected>, With<AnnotationMarker>)>,
    selected_lines: Query<&DrawnLine, (With<Selected>, With<AnnotationMarker>)>,
    selected_texts: Query<(&Transform, &TextAnnotation), (With<Selected>, With<AnnotationMarker>)>,
) {
    // Check for Ctrl+C
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    if !ctrl || !keyboard.just_pressed(KeyCode::KeyC) {
        return;
    }

    // Don't copy if UI has keyboard focus (user typing)
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    // Nothing selected? Don't clear clipboard
    if selected_items.is_empty()
        && selected_paths.is_empty()
        && selected_lines.is_empty()
        && selected_texts.is_empty()
    {
        return;
    }

    // Calculate centroid of all selected items
    let centroid = calculate_selection_centroid(
        &selected_items,
        &selected_paths,
        &selected_lines,
        &selected_texts,
    );

    // Clear clipboard
    clipboard.clear();

    // Copy placed items
    for (item, transform) in selected_items.iter() {
        let saved = SavedPlacedItem::from_entity(item, transform);
        let offset = saved.position - centroid;
        clipboard
            .placed_items
            .push(ClipboardPlacedItem { saved, offset });
    }

    // Copy paths
    for path in selected_paths.iter() {
        let center = path_center(path);
        let offset = center - centroid;
        let saved = SavedPath {
            points: path.points.clone(),
            color: color_to_array(path.color),
            stroke_width: path.stroke_width,
        };
        clipboard.paths.push(ClipboardPath { saved, offset });
    }

    // Copy lines
    for line in selected_lines.iter() {
        let line_center = (line.start + line.end) / 2.0;
        let offset = line_center - centroid;
        let saved = SavedLine {
            start: line.start,
            end: line.end,
            color: color_to_array(line.color),
            stroke_width: line.stroke_width,
        };
        clipboard.lines.push(ClipboardLine { saved, offset });
    }

    // Copy text annotations
    for (transform, text) in selected_texts.iter() {
        let pos = transform.translation.truncate();
        let offset = pos - centroid;
        let saved = SavedTextBox {
            position: pos,
            content: text.content.clone(),
            font_size: text.font_size,
            color: color_to_array(text.color),
        };
        clipboard.texts.push(ClipboardText { saved, offset });
    }
}
