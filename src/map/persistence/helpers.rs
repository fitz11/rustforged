//! Helper functions for map persistence.

use bevy::prelude::*;
use serde::Deserialize;

pub fn color_to_array(color: Color) -> [f32; 4] {
    let srgba = color.to_srgba();
    [srgba.red, srgba.green, srgba.blue, srgba.alpha]
}

pub fn array_to_color(arr: [f32; 4]) -> Color {
    Color::srgba(arr[0], arr[1], arr[2], arr[3])
}

/// Which machine saved a map file, read from its contents without parsing the whole map.
pub fn saved_by_of(map_file: &[u8]) -> Option<String> {
    #[derive(Deserialize)]
    struct SavedBy {
        saved_by: Option<String>,
    }
    serde_json::from_slice::<SavedBy>(map_file)
        .ok()
        .and_then(|s| s.saved_by)
}
