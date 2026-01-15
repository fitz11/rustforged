//! Unit tests for clipboard operations.

#![cfg(test)]

use bevy::prelude::*;

use crate::editor::annotations::DrawnPath;
use crate::editor::clipboard::helpers::{
    array_to_color, color_to_array, path_center, saved_path_center,
};
use crate::editor::clipboard::types::{
    Clipboard, ClipboardLine, ClipboardPath, ClipboardPlacedItem, ClipboardText,
};
use crate::map::{Layer, SavedLine, SavedPath, SavedPlacedItem, SavedTextBox};

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
        points: vec![Vec2::new(-50.0, -50.0), Vec2::new(50.0, 50.0)],
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
    let positions = [
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
            points: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(50.0, 50.0),
                Vec2::new(100.0, 0.0),
            ],
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
