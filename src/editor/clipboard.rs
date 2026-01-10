use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::map::{Layer, MapData, PlacedItem, SavedLine, SavedPath, SavedPlacedItem, SavedTextBox, Selected};

use super::annotations::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};
use super::params::{CameraParams, SelectedAnnotationQueries};

/// Clipboard data for a placed item with offset from selection centroid
#[derive(Clone)]
pub struct ClipboardPlacedItem {
    pub saved: SavedPlacedItem,
    pub offset: Vec2,
}

/// Clipboard data for a path annotation with offset from selection centroid
#[derive(Clone)]
pub struct ClipboardPath {
    pub saved: SavedPath,
    pub offset: Vec2,
}

/// Clipboard data for a line annotation with offset from selection centroid
#[derive(Clone)]
pub struct ClipboardLine {
    pub saved: SavedLine,
    pub offset: Vec2,
}

/// Clipboard data for a text annotation with offset from selection centroid
#[derive(Clone)]
pub struct ClipboardText {
    pub saved: SavedTextBox,
    pub offset: Vec2,
}

/// Resource that holds copied items
#[derive(Resource, Default)]
pub struct Clipboard {
    pub placed_items: Vec<ClipboardPlacedItem>,
    pub paths: Vec<ClipboardPath>,
    pub lines: Vec<ClipboardLine>,
    pub texts: Vec<ClipboardText>,
}

impl Clipboard {
    pub fn is_empty(&self) -> bool {
        self.placed_items.is_empty()
            && self.paths.is_empty()
            && self.lines.is_empty()
            && self.texts.is_empty()
    }

    pub fn clear(&mut self) {
        self.placed_items.clear();
        self.paths.clear();
        self.lines.clear();
        self.texts.clear();
    }
}

/// Convert Color to [f32; 4] array for saved formats
fn color_to_array(color: Color) -> [f32; 4] {
    let srgba = color.to_srgba();
    [srgba.red, srgba.green, srgba.blue, srgba.alpha]
}

/// Convert [f32; 4] array to Color
fn array_to_color(arr: [f32; 4]) -> Color {
    Color::srgba(arr[0], arr[1], arr[2], arr[3])
}

/// Calculate the center of a DrawnPath
fn path_center(path: &DrawnPath) -> Vec2 {
    if path.points.is_empty() {
        return Vec2::ZERO;
    }
    let sum: Vec2 = path.points.iter().copied().sum();
    sum / path.points.len() as f32
}

/// Calculate the center from saved path points
fn saved_path_center(saved: &SavedPath) -> Vec2 {
    if saved.points.is_empty() {
        return Vec2::ZERO;
    }
    let sum: Vec2 = saved.points.iter().copied().sum();
    sum / saved.points.len() as f32
}

/// Calculate the centroid of all selected items
#[allow(clippy::type_complexity)]
fn calculate_selection_centroid(
    placed_items: &Query<(&PlacedItem, &Transform), With<Selected>>,
    paths: &Query<&DrawnPath, (With<Selected>, With<AnnotationMarker>)>,
    lines: &Query<&DrawnLine, (With<Selected>, With<AnnotationMarker>)>,
    texts: &Query<(&Transform, &TextAnnotation), (With<Selected>, With<AnnotationMarker>)>,
) -> Vec2 {
    let mut positions: Vec<Vec2> = Vec::new();

    // Collect placed item positions
    for (_, transform) in placed_items.iter() {
        positions.push(transform.translation.truncate());
    }

    // Collect path centers
    for path in paths.iter() {
        positions.push(path_center(path));
    }

    // Collect line centers
    for line in lines.iter() {
        positions.push((line.start + line.end) / 2.0);
    }

    // Collect text positions
    for (transform, _) in texts.iter() {
        positions.push(transform.translation.truncate());
    }

    if positions.is_empty() {
        return Vec2::ZERO;
    }

    let sum: Vec2 = positions.iter().copied().sum();
    sum / positions.len() as f32
}

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
        clipboard.placed_items.push(ClipboardPlacedItem { saved, offset });
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
        clipboard.placed_items.push(ClipboardPlacedItem { saved, offset });
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

/// Paste clipboard items at cursor position (Ctrl+V)
#[allow(clippy::too_many_arguments)]
pub fn handle_paste(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    clipboard: Res<Clipboard>,
    mut contexts: EguiContexts,
    asset_server: Res<AssetServer>,
    camera: CameraParams,
    selected_query: Query<Entity, With<Selected>>,
    map_data: Res<MapData>,
) {
    // Check for Ctrl+V
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    if !ctrl || !keyboard.just_pressed(KeyCode::KeyV) {
        return;
    }

    // Don't paste if UI has keyboard focus
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

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

    // Paste placed items
    for clip_item in &clipboard.placed_items {
        // Check if target layer is locked
        let layer_locked = map_data
            .layers
            .iter()
            .find(|ld| ld.layer_type == clip_item.saved.layer)
            .map(|ld| ld.locked)
            .unwrap_or(false);

        if layer_locked {
            continue;
        }

        let new_pos = paste_pos + clip_item.offset;
        let z = clip_item.saved.layer.z_base() + clip_item.saved.z_index as f32;

        let texture: Handle<Image> = asset_server.load(&clip_item.saved.asset_path);

        commands.spawn((
            Sprite::from_image(texture),
            Transform {
                translation: new_pos.extend(z),
                rotation: Quat::from_rotation_z(clip_item.saved.rotation),
                scale: clip_item.saved.scale.extend(1.0),
            },
            PlacedItem {
                asset_path: clip_item.saved.asset_path.clone(),
                layer: clip_item.saved.layer,
                z_index: clip_item.saved.z_index,
            },
            Selected, // Auto-select pasted item
        ));
    }

    // Check if annotation layer is locked
    let annotation_locked = map_data
        .layers
        .iter()
        .find(|ld| ld.layer_type == Layer::Annotation)
        .map(|ld| ld.locked)
        .unwrap_or(false);

    if annotation_locked {
        return;
    }

    let annotation_z = Layer::Annotation.z_base();

    // Paste paths
    for clip_path in &clipboard.paths {
        // Translate all points to new position
        let center = saved_path_center(&clip_path.saved);
        let translation = paste_pos + clip_path.offset - center;

        let new_points: Vec<Vec2> = clip_path
            .saved
            .points
            .iter()
            .map(|p| *p + translation)
            .collect();

        commands.spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, annotation_z)),
            DrawnPath {
                points: new_points,
                color: array_to_color(clip_path.saved.color),
                stroke_width: clip_path.saved.stroke_width,
            },
            AnnotationMarker,
            Selected,
        ));
    }

    // Paste lines
    for clip_line in &clipboard.lines {
        let line_center = (clip_line.saved.start + clip_line.saved.end) / 2.0;
        let translation = paste_pos + clip_line.offset - line_center;

        commands.spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, annotation_z)),
            DrawnLine {
                start: clip_line.saved.start + translation,
                end: clip_line.saved.end + translation,
                color: array_to_color(clip_line.saved.color),
                stroke_width: clip_line.saved.stroke_width,
            },
            AnnotationMarker,
            Selected,
        ));
    }

    // Paste text annotations
    for clip_text in &clipboard.texts {
        let new_pos = paste_pos + clip_text.offset;

        commands.spawn((
            Transform::from_translation(new_pos.extend(annotation_z)),
            TextAnnotation {
                content: clip_text.saved.content.clone(),
                font_size: clip_text.saved.font_size,
                color: array_to_color(clip_text.saved.color),
            },
            AnnotationMarker,
            Selected,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Clipboard resource tests
    #[test]
    fn test_clipboard_default_is_empty() {
        let clipboard = Clipboard::default();
        assert!(clipboard.is_empty());
    }

    #[test]
    fn test_clipboard_is_empty_with_placed_item() {
        let mut clipboard = Clipboard::default();
        clipboard.placed_items.push(ClipboardPlacedItem {
            saved: SavedPlacedItem {
                asset_path: "test.png".to_string(),
                position: Vec2::ZERO,
                rotation: 0.0,
                scale: Vec2::ONE,
                layer: Layer::Token,
                z_index: 0,
            },
            offset: Vec2::ZERO,
        });
        assert!(!clipboard.is_empty());
    }

    #[test]
    fn test_clipboard_is_empty_with_path() {
        let mut clipboard = Clipboard::default();
        clipboard.paths.push(ClipboardPath {
            saved: SavedPath {
                points: vec![Vec2::ZERO, Vec2::ONE],
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_width: 2.0,
            },
            offset: Vec2::ZERO,
        });
        assert!(!clipboard.is_empty());
    }

    #[test]
    fn test_clipboard_is_empty_with_line() {
        let mut clipboard = Clipboard::default();
        clipboard.lines.push(ClipboardLine {
            saved: SavedLine {
                start: Vec2::ZERO,
                end: Vec2::new(100.0, 100.0),
                color: [0.0, 1.0, 0.0, 1.0],
                stroke_width: 3.0,
            },
            offset: Vec2::ZERO,
        });
        assert!(!clipboard.is_empty());
    }

    #[test]
    fn test_clipboard_is_empty_with_text() {
        let mut clipboard = Clipboard::default();
        clipboard.texts.push(ClipboardText {
            saved: SavedTextBox {
                position: Vec2::new(50.0, 50.0),
                content: "Hello".to_string(),
                font_size: 16.0,
                color: [1.0, 1.0, 1.0, 1.0],
            },
            offset: Vec2::ZERO,
        });
        assert!(!clipboard.is_empty());
    }

    #[test]
    fn test_clipboard_clear() {
        let mut clipboard = Clipboard::default();

        // Add items of each type
        clipboard.placed_items.push(ClipboardPlacedItem {
            saved: SavedPlacedItem {
                asset_path: "test.png".to_string(),
                position: Vec2::ZERO,
                rotation: 0.0,
                scale: Vec2::ONE,
                layer: Layer::Token,
                z_index: 0,
            },
            offset: Vec2::ZERO,
        });
        clipboard.paths.push(ClipboardPath {
            saved: SavedPath {
                points: vec![Vec2::ZERO],
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_width: 2.0,
            },
            offset: Vec2::ZERO,
        });
        clipboard.lines.push(ClipboardLine {
            saved: SavedLine {
                start: Vec2::ZERO,
                end: Vec2::ONE,
                color: [0.0, 1.0, 0.0, 1.0],
                stroke_width: 3.0,
            },
            offset: Vec2::ZERO,
        });
        clipboard.texts.push(ClipboardText {
            saved: SavedTextBox {
                position: Vec2::ZERO,
                content: "Test".to_string(),
                font_size: 12.0,
                color: [1.0, 1.0, 1.0, 1.0],
            },
            offset: Vec2::ZERO,
        });

        assert!(!clipboard.is_empty());

        clipboard.clear();

        assert!(clipboard.is_empty());
        assert!(clipboard.placed_items.is_empty());
        assert!(clipboard.paths.is_empty());
        assert!(clipboard.lines.is_empty());
        assert!(clipboard.texts.is_empty());
    }

    // Color conversion tests
    #[test]
    fn test_color_to_array() {
        let color = Color::srgba(1.0, 0.5, 0.25, 0.75);
        let arr = color_to_array(color);

        assert!((arr[0] - 1.0).abs() < 0.001);
        assert!((arr[1] - 0.5).abs() < 0.001);
        assert!((arr[2] - 0.25).abs() < 0.001);
        assert!((arr[3] - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_array_to_color() {
        let arr = [0.2, 0.4, 0.6, 0.8];
        let color = array_to_color(arr);
        let srgba = color.to_srgba();

        assert!((srgba.red - 0.2).abs() < 0.001);
        assert!((srgba.green - 0.4).abs() < 0.001);
        assert!((srgba.blue - 0.6).abs() < 0.001);
        assert!((srgba.alpha - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_color_roundtrip() {
        let original = Color::srgba(0.1, 0.2, 0.3, 0.4);
        let arr = color_to_array(original);
        let restored = array_to_color(arr);

        let orig_srgba = original.to_srgba();
        let rest_srgba = restored.to_srgba();

        assert!((orig_srgba.red - rest_srgba.red).abs() < 0.001);
        assert!((orig_srgba.green - rest_srgba.green).abs() < 0.001);
        assert!((orig_srgba.blue - rest_srgba.blue).abs() < 0.001);
        assert!((orig_srgba.alpha - rest_srgba.alpha).abs() < 0.001);
    }

    // Path center tests
    #[test]
    fn test_path_center_empty() {
        let path = DrawnPath {
            points: vec![],
            color: Color::WHITE,
            stroke_width: 1.0,
        };
        assert_eq!(path_center(&path), Vec2::ZERO);
    }

    #[test]
    fn test_path_center_single_point() {
        let path = DrawnPath {
            points: vec![Vec2::new(100.0, 200.0)],
            color: Color::WHITE,
            stroke_width: 1.0,
        };
        assert_eq!(path_center(&path), Vec2::new(100.0, 200.0));
    }

    #[test]
    fn test_path_center_multiple_points() {
        let path = DrawnPath {
            points: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(100.0, 0.0),
                Vec2::new(100.0, 100.0),
                Vec2::new(0.0, 100.0),
            ],
            color: Color::WHITE,
            stroke_width: 1.0,
        };
        // Average of corners should be center
        assert_eq!(path_center(&path), Vec2::new(50.0, 50.0));
    }

    #[test]
    fn test_saved_path_center_empty() {
        let saved = SavedPath {
            points: vec![],
            color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 1.0,
        };
        assert_eq!(saved_path_center(&saved), Vec2::ZERO);
    }

    #[test]
    fn test_saved_path_center_multiple_points() {
        let saved = SavedPath {
            points: vec![
                Vec2::new(-50.0, -50.0),
                Vec2::new(50.0, 50.0),
            ],
            color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 1.0,
        };
        assert_eq!(saved_path_center(&saved), Vec2::ZERO);
    }

    // Clipboard item offset tests
    #[test]
    fn test_clipboard_placed_item_offset_calculation() {
        // Simulate copying an item at (100, 100) with centroid at (50, 50)
        let item_pos = Vec2::new(100.0, 100.0);
        let centroid = Vec2::new(50.0, 50.0);
        let offset = item_pos - centroid;

        assert_eq!(offset, Vec2::new(50.0, 50.0));

        // When pasting at (200, 200), item should be at (250, 250)
        let paste_pos = Vec2::new(200.0, 200.0);
        let new_pos = paste_pos + offset;

        assert_eq!(new_pos, Vec2::new(250.0, 250.0));
    }

    #[test]
    fn test_clipboard_preserves_relative_positions() {
        // Three items at (0,0), (100,0), (50,100)
        // Centroid = (50, 33.33...)
        let positions = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(50.0, 100.0),
        ];

        let centroid = positions.iter().copied().sum::<Vec2>() / positions.len() as f32;

        // Calculate offsets
        let offsets: Vec<Vec2> = positions.iter().map(|p| *p - centroid).collect();

        // Paste at (500, 500)
        let paste_pos = Vec2::new(500.0, 500.0);
        let new_positions: Vec<Vec2> = offsets.iter().map(|o| paste_pos + *o).collect();

        // Check relative distances are preserved
        let orig_dist_01 = (positions[0] - positions[1]).length();
        let new_dist_01 = (new_positions[0] - new_positions[1]).length();
        assert!((orig_dist_01 - new_dist_01).abs() < 0.001);

        let orig_dist_12 = (positions[1] - positions[2]).length();
        let new_dist_12 = (new_positions[1] - new_positions[2]).length();
        assert!((orig_dist_12 - new_dist_12).abs() < 0.001);
    }

    #[test]
    fn test_line_center_calculation() {
        let start = Vec2::new(0.0, 0.0);
        let end = Vec2::new(100.0, 100.0);
        let center = (start + end) / 2.0;

        assert_eq!(center, Vec2::new(50.0, 50.0));
    }

    // ClipboardPlacedItem tests
    #[test]
    fn test_clipboard_placed_item_clone() {
        let item = ClipboardPlacedItem {
            saved: SavedPlacedItem {
                asset_path: "tokens/hero.png".to_string(),
                position: Vec2::new(100.0, 200.0),
                rotation: std::f32::consts::PI / 4.0,
                scale: Vec2::new(2.0, 2.0),
                layer: Layer::Token,
                z_index: 5,
            },
            offset: Vec2::new(10.0, 20.0),
        };

        let cloned = item.clone();

        assert_eq!(cloned.saved.asset_path, "tokens/hero.png");
        assert_eq!(cloned.saved.position, Vec2::new(100.0, 200.0));
        assert_eq!(cloned.saved.layer, Layer::Token);
        assert_eq!(cloned.offset, Vec2::new(10.0, 20.0));
    }

    // ClipboardPath tests
    #[test]
    fn test_clipboard_path_clone() {
        let path = ClipboardPath {
            saved: SavedPath {
                points: vec![Vec2::new(0.0, 0.0), Vec2::new(50.0, 50.0), Vec2::new(100.0, 0.0)],
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_width: 3.0,
            },
            offset: Vec2::new(-25.0, 15.0),
        };

        let cloned = path.clone();

        assert_eq!(cloned.saved.points.len(), 3);
        assert_eq!(cloned.saved.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(cloned.saved.stroke_width, 3.0);
        assert_eq!(cloned.offset, Vec2::new(-25.0, 15.0));
    }

    // ClipboardLine tests
    #[test]
    fn test_clipboard_line_clone() {
        let line = ClipboardLine {
            saved: SavedLine {
                start: Vec2::new(10.0, 20.0),
                end: Vec2::new(110.0, 120.0),
                color: [0.0, 1.0, 0.0, 0.5],
                stroke_width: 5.0,
            },
            offset: Vec2::new(30.0, 40.0),
        };

        let cloned = line.clone();

        assert_eq!(cloned.saved.start, Vec2::new(10.0, 20.0));
        assert_eq!(cloned.saved.end, Vec2::new(110.0, 120.0));
        assert_eq!(cloned.saved.color, [0.0, 1.0, 0.0, 0.5]);
        assert_eq!(cloned.offset, Vec2::new(30.0, 40.0));
    }

    // ClipboardText tests
    #[test]
    fn test_clipboard_text_clone() {
        let text = ClipboardText {
            saved: SavedTextBox {
                position: Vec2::new(200.0, 300.0),
                content: "Test annotation".to_string(),
                font_size: 24.0,
                color: [0.0, 0.0, 1.0, 1.0],
            },
            offset: Vec2::new(-50.0, -60.0),
        };

        let cloned = text.clone();

        assert_eq!(cloned.saved.position, Vec2::new(200.0, 300.0));
        assert_eq!(cloned.saved.content, "Test annotation");
        assert_eq!(cloned.saved.font_size, 24.0);
        assert_eq!(cloned.offset, Vec2::new(-50.0, -60.0));
    }

    // Mixed clipboard content tests
    #[test]
    fn test_clipboard_with_mixed_content() {
        let mut clipboard = Clipboard::default();

        // Add one of each type
        clipboard.placed_items.push(ClipboardPlacedItem {
            saved: SavedPlacedItem {
                asset_path: "test.png".to_string(),
                position: Vec2::new(0.0, 0.0),
                rotation: 0.0,
                scale: Vec2::ONE,
                layer: Layer::Doodad,
                z_index: 0,
            },
            offset: Vec2::new(10.0, 0.0),
        });

        clipboard.paths.push(ClipboardPath {
            saved: SavedPath {
                points: vec![Vec2::new(50.0, 50.0)],
                color: [1.0, 0.0, 0.0, 1.0],
                stroke_width: 2.0,
            },
            offset: Vec2::new(-10.0, 0.0),
        });

        assert!(!clipboard.is_empty());
        assert_eq!(clipboard.placed_items.len(), 1);
        assert_eq!(clipboard.paths.len(), 1);
        assert_eq!(clipboard.lines.len(), 0);
        assert_eq!(clipboard.texts.len(), 0);
    }
}
