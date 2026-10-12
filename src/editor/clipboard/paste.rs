//! Paste system for clipboard operations (Ctrl+V).

use bevy::prelude::*;

use crate::map::{ItemId, Layer, MapData, Selected, StackTops};

use super::super::annotations::{DrawnLine, DrawnPath, TextAnnotation};
use super::super::history::{EditorCommand, MapObject, RecordEditorCommand};
use super::super::params::CameraParams;
use super::helpers::{array_to_color, saved_path_center};
use super::types::Clipboard;

/// Paste clipboard items at cursor position (Ctrl+V)
#[allow(clippy::too_many_arguments)]
pub fn handle_paste(
    mut commands: Commands,
    clipboard: Res<Clipboard>,
    asset_server: Res<AssetServer>,
    camera: CameraParams,
    selected_query: Query<Entity, With<Selected>>,
    map_data: Res<MapData>,
    mut history_writer: MessageWriter<RecordEditorCommand>,
    stack: StackTops,
) {
    // Check clipboard has content
    if clipboard.is_empty() {
        return;
    }

    // Get cursor world position
    let Some(paste_pos) = camera.cursor_world_pos() else {
        return;
    };

    // Clear current selection
    for entity in selected_query.iter() {
        commands.entity(entity).remove::<Selected>();
    }

    // Everything pasted is one undo step
    let mut pasted: Vec<MapObject> = Vec::new();
    // Pasted items go on top, keeping their order (the clipboard lists them bottom first)
    let mut stacker = stack.stacker();

    // Paste placed items
    for clip_item in &clipboard.placed_items {
        // Check if target layer is locked
        if map_data.is_layer_locked(clip_item.saved.layer) {
            continue;
        }
        let item = clip_item.saved.placed_item();
        let transform = Transform {
            translation: (paste_pos + clip_item.offset).extend(stacker.next_z(&item)),
            ..clip_item.saved.transform()
        };
        pasted.push(MapObject::new_item(item, transform));
    }

    if !map_data.is_layer_locked(Layer::Annotation) {
        // Paste paths
        for clip_path in &clipboard.paths {
            // Translate all points to new position
            let center = saved_path_center(&clip_path.saved);
            let translation = paste_pos + clip_path.offset - center;
            pasted.push(MapObject::new_path(DrawnPath {
                points: clip_path
                    .saved
                    .points
                    .iter()
                    .map(|p| *p + translation)
                    .collect(),
                color: array_to_color(clip_path.saved.color),
                stroke_width: clip_path.saved.stroke_width,
            }));
        }

        // Paste lines
        for clip_line in &clipboard.lines {
            let line_center = (clip_line.saved.start + clip_line.saved.end) / 2.0;
            let translation = paste_pos + clip_line.offset - line_center;
            pasted.push(MapObject::new_line(DrawnLine {
                start: clip_line.saved.start + translation,
                end: clip_line.saved.end + translation,
                color: array_to_color(clip_line.saved.color),
                stroke_width: clip_line.saved.stroke_width,
            }));
        }

        // Paste text annotations
        for clip_text in &clipboard.texts {
            pasted.push(MapObject::Text {
                id: ItemId::new(),
                text: TextAnnotation {
                    content: clip_text.saved.content.clone(),
                    font_size: clip_text.saved.font_size,
                    color: array_to_color(clip_text.saved.color),
                },
                position: paste_pos + clip_text.offset,
            });
        }
    }

    if pasted.is_empty() {
        return;
    }
    // Select what was pasted
    for object in &pasted {
        let entity = object.spawn(&mut commands, &asset_server);
        commands.entity(entity).insert(Selected);
    }
    history_writer.write(RecordEditorCommand {
        command: EditorCommand::Add(pasted),
    });
}
