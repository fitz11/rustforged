//! Helper functions for map persistence.

use bevy::prelude::*;
use std::path::PathBuf;

pub fn color_to_array(color: Color) -> [f32; 4] {
    let srgba = color.to_srgba();
    [srgba.red, srgba.green, srgba.blue, srgba.alpha]
}

pub fn array_to_color(arr: [f32; 4]) -> Color {
    Color::srgba(arr[0], arr[1], arr[2], arr[3])
}

pub fn ensure_maps_directory() {
    let maps_dir = PathBuf::from("assets/maps");
    if !maps_dir.exists()
        && let Err(e) = std::fs::create_dir_all(&maps_dir)
    {
        warn!("Failed to create maps directory: {}", e);
    }
}
