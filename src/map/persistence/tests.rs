//! Unit tests for the persistence module.

use bevy::prelude::*;

use super::helpers::{array_to_color, color_to_array};
use super::resources::MapLoadError;

// color_to_array tests
#[test]
fn test_color_to_array_red() {
    let color = Color::srgba(1.0, 0.0, 0.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_color_to_array_green() {
    let color = Color::srgba(0.0, 1.0, 0.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn test_color_to_array_blue() {
    let color = Color::srgba(0.0, 0.0, 1.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn test_color_to_array_with_alpha() {
    let color = Color::srgba(0.5, 0.5, 0.5, 0.5);
    let arr = color_to_array(color);
    assert!((arr[0] - 0.5).abs() < 0.001);
    assert!((arr[1] - 0.5).abs() < 0.001);
    assert!((arr[2] - 0.5).abs() < 0.001);
    assert!((arr[3] - 0.5).abs() < 0.001);
}

#[test]
fn test_color_to_array_white() {
    let color = Color::srgba(1.0, 1.0, 1.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn test_color_to_array_black() {
    let color = Color::srgba(0.0, 0.0, 0.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [0.0, 0.0, 0.0, 1.0]);
}

// array_to_color tests
#[test]
fn test_array_to_color_red() {
    let arr = [1.0, 0.0, 0.0, 1.0];
    let color = array_to_color(arr);
    let srgba = color.to_srgba();
    assert_eq!(srgba.red, 1.0);
    assert_eq!(srgba.green, 0.0);
    assert_eq!(srgba.blue, 0.0);
    assert_eq!(srgba.alpha, 1.0);
}

#[test]
fn test_array_to_color_partial_alpha() {
    let arr = [0.25, 0.5, 0.75, 0.5];
    let color = array_to_color(arr);
    let srgba = color.to_srgba();
    assert!((srgba.red - 0.25).abs() < 0.001);
    assert!((srgba.green - 0.5).abs() < 0.001);
    assert!((srgba.blue - 0.75).abs() < 0.001);
    assert!((srgba.alpha - 0.5).abs() < 0.001);
}

// Round-trip tests
#[test]
fn test_color_roundtrip() {
    let original = Color::srgba(0.2, 0.4, 0.6, 0.8);
    let arr = color_to_array(original);
    let recovered = array_to_color(arr);
    let original_srgba = original.to_srgba();
    let recovered_srgba = recovered.to_srgba();

    assert!((original_srgba.red - recovered_srgba.red).abs() < 0.001);
    assert!((original_srgba.green - recovered_srgba.green).abs() < 0.001);
    assert!((original_srgba.blue - recovered_srgba.blue).abs() < 0.001);
    assert!((original_srgba.alpha - recovered_srgba.alpha).abs() < 0.001);
}

#[test]
fn test_color_roundtrip_multiple() {
    let colors = [
        Color::srgba(1.0, 0.0, 0.0, 1.0),
        Color::srgba(0.0, 1.0, 0.0, 1.0),
        Color::srgba(0.0, 0.0, 1.0, 1.0),
        Color::srgba(1.0, 1.0, 1.0, 1.0),
        Color::srgba(0.0, 0.0, 0.0, 1.0),
        Color::srgba(0.5, 0.5, 0.5, 0.5),
        Color::srgba(0.1, 0.2, 0.3, 0.4),
    ];

    for original in colors {
        let arr = color_to_array(original);
        let recovered = array_to_color(arr);
        let original_srgba = original.to_srgba();
        let recovered_srgba = recovered.to_srgba();

        assert!(
            (original_srgba.red - recovered_srgba.red).abs() < 0.001,
            "Red mismatch for {:?}",
            original
        );
        assert!(
            (original_srgba.green - recovered_srgba.green).abs() < 0.001,
            "Green mismatch for {:?}",
            original
        );
        assert!(
            (original_srgba.blue - recovered_srgba.blue).abs() < 0.001,
            "Blue mismatch for {:?}",
            original
        );
        assert!(
            (original_srgba.alpha - recovered_srgba.alpha).abs() < 0.001,
            "Alpha mismatch for {:?}",
            original
        );
    }
}

// MapLoadError tests
#[test]
fn test_map_load_error_default() {
    let error = MapLoadError::default();
    assert!(error.message.is_none());
}
