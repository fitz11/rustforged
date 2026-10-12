//! Keyboard shortcuts for selection operations.

use bevy::prelude::*;

use crate::editor::brush::fit_to_cell_scale;
use crate::editor::grid::snap_to_grid;
use crate::editor::history::{EditorCommand, MapObject, RecordEditorCommand, remove_from_map};
use crate::editor::params::SelectedObjects;
use crate::input::{plain_key_just_pressed, shift_held};
use crate::map::{ItemId, MapData, PlacedItem, Selected};

use super::hit_detection::get_sprite_half_size;

/// Selected placed items, for the transform shortcuts
type SelectedItems<'w, 's> = Query<
    'w,
    's,
    (
        &'static ItemId,
        &'static PlacedItem,
        &'static mut Transform,
        &'static Sprite,
    ),
    With<Selected>,
>;

/// Applies `edit` to every selected placed item and records the result as one undo step.
fn edit_selected_items(
    selected: &mut SelectedItems,
    history_writer: &mut MessageWriter<RecordEditorCommand>,
    mut edit: impl FnMut(&Sprite, &mut Transform),
) {
    let mut pairs = Vec::new();
    for (id, item, mut transform, sprite) in selected.iter_mut() {
        let before = MapObject::item(*id, item.clone(), *transform);
        edit(sprite, &mut transform);
        pairs.push((before, MapObject::item(*id, item.clone(), *transform)));
    }
    if let Some(command) = EditorCommand::change(pairs) {
        history_writer.write(RecordEditorCommand { command });
    }
}

/// An edit to every selected placed item, from a keyboard shortcut or the Properties panel.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionEdit {
    /// Scale to fit one grid cell (G)
    FitToGrid,
    /// Move to the center of the grid cell (Shift+G)
    CenterToGrid,
    /// Make the scale uniform again, keeping the larger side (A)
    RestoreAspectRatio,
    /// Turn by 90 degrees (R clockwise, Shift+R counter-clockwise)
    Rotate90 { clockwise: bool },
}

/// G, Shift+G, A, R and Shift+R
pub fn selection_edit_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut edits: MessageWriter<SelectionEdit>,
) {
    let shift = shift_held(&keyboard);
    let pressed = |key| plain_key_just_pressed(&keyboard, key);
    if pressed(KeyCode::KeyG) {
        edits.write(if shift {
            SelectionEdit::CenterToGrid
        } else {
            SelectionEdit::FitToGrid
        });
    }
    if pressed(KeyCode::KeyA) {
        edits.write(SelectionEdit::RestoreAspectRatio);
    }
    if pressed(KeyCode::KeyR) {
        edits.write(SelectionEdit::Rotate90 { clockwise: !shift });
    }
}

/// Applies [`SelectionEdit`]s, each as one undo step.
pub fn apply_selection_edits(
    mut edits: MessageReader<SelectionEdit>,
    mut selected: SelectedItems,
    map_data: Res<MapData>,
    images: Res<Assets<Image>>,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    let grid_size = map_data.grid_size;
    for edit in edits.read() {
        edit_selected_items(&mut selected, &mut history_writer, |sprite, transform| {
            match edit {
                SelectionEdit::FitToGrid => {
                    let original_size = get_sprite_half_size(sprite, &images) * 2.0;
                    if original_size.x > 0.0 && original_size.y > 0.0 {
                        transform.scale = fit_to_cell_scale(original_size, grid_size);
                    }
                }
                SelectionEdit::CenterToGrid => {
                    let snapped = snap_to_grid(transform.translation.truncate(), grid_size, true);
                    transform.translation.x = snapped.x;
                    transform.translation.y = snapped.y;
                }
                SelectionEdit::RestoreAspectRatio => {
                    // Use the larger scale value to preserve the largest dimension
                    let uniform_scale = transform.scale.x.abs().max(transform.scale.y.abs());
                    transform.scale.x = uniform_scale;
                    transform.scale.y = uniform_scale;
                }
                SelectionEdit::Rotate90 { clockwise } => {
                    let angle = if *clockwise { -90.0_f32 } else { 90.0_f32 };
                    transform.rotation *= Quat::from_rotation_z(angle.to_radians());
                }
            }
        });
    }
}

/// Delete the selection (Delete or Backspace) as one undo step
pub fn handle_deletion(
    mut commands: Commands,
    selected: SelectedObjects,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    remove_from_map(&mut commands, &mut history_writer, selected.collect());
}

/// Clear selection when Escape is pressed
pub fn handle_escape_clear_selection(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_query: Query<Entity, With<Selected>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        for entity in selected_query.iter() {
            commands.entity(entity).remove::<Selected>();
        }
    }
}
