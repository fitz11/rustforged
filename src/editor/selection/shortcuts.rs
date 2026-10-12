//! Keyboard shortcuts for selection operations.

use bevy::prelude::*;

use crate::editor::brush::fit_to_cell_scale;
use crate::editor::grid::snap_to_grid;
use crate::editor::history::{EditorCommand, MapObject, RecordEditorCommand};
use crate::editor::params::SelectedObjects;
use crate::input::shift_held;
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

/// Scale selected items to fit one grid cell when G is pressed
pub fn handle_fit_to_grid(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: SelectedItems,
    map_data: Res<MapData>,
    images: Res<Assets<Image>>,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    // G = fit to grid (but not Shift+G, which is center to grid)
    if shift_held(&keyboard) {
        return;
    }
    let grid_size = map_data.grid_size;
    edit_selected_items(&mut selected, &mut history_writer, |sprite, transform| {
        let original_size = get_sprite_half_size(sprite, &images) * 2.0;
        if original_size.x > 0.0 && original_size.y > 0.0 {
            transform.scale = fit_to_cell_scale(original_size, grid_size);
        }
    });
}

/// Center selected items to the nearest grid cell center when Shift+G is pressed
pub fn handle_center_to_grid(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: SelectedItems,
    map_data: Res<MapData>,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    if !shift_held(&keyboard) {
        return;
    }
    let grid_size = map_data.grid_size;
    edit_selected_items(&mut selected, &mut history_writer, |_, transform| {
        let snapped = snap_to_grid(transform.translation.truncate(), grid_size, true);
        transform.translation.x = snapped.x;
        transform.translation.y = snapped.y;
    });
}

/// Restore selected items to their original aspect ratio when A is pressed
/// Uses the larger of the two scale values to preserve the largest dimension
pub fn handle_restore_aspect_ratio(
    mut selected: SelectedItems,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    edit_selected_items(&mut selected, &mut history_writer, |_, transform| {
        let uniform_scale = transform.scale.x.abs().max(transform.scale.y.abs());
        transform.scale.x = uniform_scale;
        transform.scale.y = uniform_scale;
    });
}

/// Rotate selected items by 90 degrees when R is pressed (clockwise) or Shift+R (counter-clockwise)
pub fn handle_rotate_90(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: SelectedItems,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    // Rotate 90 degrees: clockwise (negative) or counter-clockwise (positive) with Shift
    let angle = if shift_held(&keyboard) {
        90.0_f32
    } else {
        -90.0_f32
    };
    let rotation_delta = Quat::from_rotation_z(angle.to_radians());
    edit_selected_items(&mut selected, &mut history_writer, |_, transform| {
        transform.rotation *= rotation_delta;
    });
}

/// Delete the selection (Delete or Backspace) as one undo step
pub fn handle_deletion(
    mut commands: Commands,
    selected: SelectedObjects,
    mut history_writer: MessageWriter<RecordEditorCommand>,
) {
    let (entities, objects): (Vec<_>, Vec<_>) = selected.collect().into_iter().unzip();
    if objects.is_empty() {
        return;
    }
    history_writer.write(RecordEditorCommand {
        command: EditorCommand::Remove(objects),
    });
    for entity in entities {
        commands.entity(entity).despawn();
    }
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
