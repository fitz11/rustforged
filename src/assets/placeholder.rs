//! Placeholder texture for missing assets.
//!
//! Generates a red X placeholder image to display when asset files
//! are missing or fail to load.

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Resource holding the placeholder texture for missing assets.
#[derive(Resource)]
pub struct PlaceholderTexture {
    pub handle: Handle<Image>,
}

/// Default placeholder size in pixels.
pub const PLACEHOLDER_SIZE: u32 = 64;

/// Create a red placeholder image with an X pattern.
pub fn create_placeholder_image() -> Image {
    let size = PLACEHOLDER_SIZE as usize;
    let mut data = vec![0u8; size * size * 4];

    let red: [u8; 4] = [200, 50, 50, 255];
    let dark_red: [u8; 4] = [120, 30, 30, 255];

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;

            // Draw border (2px)
            let is_border = x < 2 || x >= size - 2 || y < 2 || y >= size - 2;

            // Draw X pattern (diagonal lines)
            let on_diagonal = (x as i32 - y as i32).abs() <= 2
                || ((size - 1 - x) as i32 - y as i32).abs() <= 2;

            let color = if is_border || on_diagonal {
                red
            } else {
                dark_red
            };
            data[idx..idx + 4].copy_from_slice(&color);
        }
    }

    Image::new(
        Extent3d {
            width: PLACEHOLDER_SIZE,
            height: PLACEHOLDER_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

/// Startup system to create and register the placeholder texture.
pub fn setup_placeholder_texture(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = create_placeholder_image();
    let handle = images.add(image);
    commands.insert_resource(PlaceholderTexture { handle });
}
