//! Fog drawn as a texture: one texel per grid cell, covering the cells in view, on a
//! sprite over the map. One overlay for the editor (see-through, its opacity set in the Fog
//! tool) and one for the player window (opaque).
//!
//! The texture is rebuilt only when what it shows changes: the fog, the cells in view, the
//! grid size or the color. Before, fog was drawn with gizmo lines every frame, about 140
//! per fogged cell, which cost a lot zoomed out and left see-through stripes between lines
//! when the player view was zoomed in on a large screen.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::map::{FogOfWarData, Layer, MapData, render_layer};
use crate::session::{LiveSessionState, PlayerMapView};
use crate::theme;

use super::fog::{FogState, is_fog_layer_visible};
use super::params::CameraWithProjection;

/// Most cells along either side of the overlay; a view showing more draws no fog rather
/// than a huge texture (only reachable with a tiny grid at the farthest zoom)
const MAX_OVERLAY_CELLS: i32 = 4096;
/// Cells of fog drawn past each edge of the view, so a camera move doesn't show a gap
/// before the texture is rebuilt
const PADDING_CELLS: i32 = 2;

/// Which view an overlay draws fog for
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FogView {
    Editor,
    Player,
}

/// The fog sprite for one view, and what its texture currently shows.
#[derive(Component)]
pub struct FogOverlay {
    view: FogView,
    shown: Option<OverlayKey>,
}

/// What an overlay's texture shows; the texture is rebuilt when this changes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct OverlayKey {
    min: IVec2,
    max: IVec2,
    grid_size: f32,
    color: [u8; 4],
}

/// Spawns the editor's and the player view's fog overlays, hidden until there's fog to show.
pub fn spawn_fog_overlays(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    for (view, layer) in [
        (FogView::Editor, render_layer::EDITOR),
        (FogView::Player, render_layer::PLAYER),
    ] {
        commands.spawn((
            FogOverlay { view, shown: None },
            Sprite::from_image(images.add(fog_image(UVec2::ONE, vec![0; 4]))),
            Transform::from_xyz(0.0, 0.0, Layer::FogOfWar.z_base()),
            Visibility::Hidden,
            RenderLayers::layer(layer),
        ));
    }
}

/// Keeps both overlays showing the fog over what their views show.
#[allow(clippy::too_many_arguments)]
pub fn update_fog_overlays(
    mut overlays: Query<(
        &mut FogOverlay,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
    )>,
    mut images: ResMut<Assets<Image>>,
    map_data: Res<MapData>,
    fog: Res<FogOfWarData>,
    fog_state: Res<FogState>,
    player_view: PlayerMapView,
    session_state: Res<LiveSessionState>,
    camera: CameraWithProjection,
) {
    for (mut overlay, mut sprite, mut transform, mut visibility) in &mut overlays {
        // What this view shows, if it shows fog at all
        let source = match overlay.view {
            FogView::Editor => (is_fog_layer_visible(&map_data) && fog_state.show_in_editor)
                .then(|| camera.visible_world_rect())
                .flatten()
                .map(|rect| {
                    let base = theme::FOG_EDITOR_BASE.to_srgba();
                    let color =
                        Color::srgba(base.red, base.green, base.blue, fog_state.editor_opacity);
                    (rect, map_data.grid_size, &*fog, color, fog.is_changed())
                }),
            FogView::Player => {
                // The fog players see: frozen along with the rest of the map while frozen
                let shown = player_view.map_data();
                (session_state.is_active && is_fog_layer_visible(shown)).then(|| {
                    (
                        player_view.viewport.world_bounds(),
                        shown.grid_size,
                        player_view.fog(),
                        theme::FOG_PLAYER,
                        player_view.is_changed(),
                    )
                })
            }
        };

        let Some(((min, max), grid_size, fog, color, fog_changed)) = source else {
            visibility.set_if_neq(Visibility::Hidden);
            overlay.shown = None;
            continue;
        };
        let Some(key) = overlay_key(min, max, grid_size, color) else {
            visibility.set_if_neq(Visibility::Hidden);
            overlay.shown = None;
            continue;
        };
        visibility.set_if_neq(Visibility::Inherited);
        if overlay.shown == Some(key) && !fog_changed {
            continue;
        }

        let size = (key.max - key.min + IVec2::ONE).as_uvec2();
        let pixels = fog_pixels(fog, key.min, key.max, key.color);
        if let Some(mut image) = images.get_mut(&sprite.image) {
            *image = fog_image(size, pixels);
        }
        sprite.custom_size = Some(size.as_vec2() * grid_size);
        // The cell range runs from min's lower-left corner to max's upper-right
        let center = (key.min.as_vec2() + (key.max + IVec2::ONE).as_vec2()) * grid_size / 2.0;
        transform.translation = center.extend(Layer::FogOfWar.z_base());
        overlay.shown = Some(key);
    }
}

/// The cells to cover for a view of the world rectangle from `min` to `max`, or None if
/// there are too many to draw.
fn overlay_key(min: Vec2, max: Vec2, grid_size: f32, color: Color) -> Option<OverlayKey> {
    let to_cell = |p: Vec2| IVec2::from(crate::map::world_to_cell(p, grid_size));
    let min = to_cell(min) - IVec2::splat(PADDING_CELLS);
    let max = to_cell(max) + IVec2::splat(PADDING_CELLS);
    let size = max - min + IVec2::ONE;
    if size.x > MAX_OVERLAY_CELLS || size.y > MAX_OVERLAY_CELLS {
        return None;
    }
    Some(OverlayKey {
        min,
        max,
        grid_size,
        color: color.to_srgba().to_u8_array(),
    })
}

/// RGBA texels for the cells from `min` to `max`: `color` where there's fog, clear where
/// it's revealed. The first row is the top (highest y) of the range.
fn fog_pixels(fog: &FogOfWarData, min: IVec2, max: IVec2, color: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(((max.x - min.x + 1) * (max.y - min.y + 1) * 4) as usize);
    for y in (min.y..=max.y).rev() {
        for x in min.x..=max.x {
            if fog.is_cell_revealed((x, y)) {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                pixels.extend_from_slice(&color);
            }
        }
    }
    pixels
}

/// A texture of `size` cells, sampled without smoothing so each cell has a hard edge.
fn fog_image(size: UVec2, pixels: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fog_pixels_rows_run_top_down() {
        let mut fog = FogOfWarData::default();
        // Bottom-left cell revealed
        fog.reveal_cell((0, 0));
        let black = [0, 0, 0, 255];

        let pixels = fog_pixels(&fog, IVec2::new(0, 0), IVec2::new(1, 1), black);

        // Top row (y = 1): both fogged; bottom row (y = 0): revealed, then fogged
        assert_eq!(&pixels[0..8], &[0, 0, 0, 255, 0, 0, 0, 255]);
        assert_eq!(&pixels[8..16], &[0, 0, 0, 0, 0, 0, 0, 255]);
    }

    #[test]
    fn test_overlay_covers_the_view_with_padding() {
        let key = overlay_key(
            Vec2::new(0.0, 0.0),
            Vec2::new(139.0, 69.0),
            70.0,
            Color::BLACK,
        )
        .unwrap();
        assert_eq!(key.min, IVec2::splat(-PADDING_CELLS));
        assert_eq!(key.max, IVec2::new(1, 0) + IVec2::splat(PADDING_CELLS));
    }

    #[test]
    fn test_overlay_too_big_draws_nothing() {
        let huge = MAX_OVERLAY_CELLS as f32 * 10.0;
        assert!(overlay_key(Vec2::ZERO, Vec2::splat(huge * 2.0), 10.0, Color::BLACK).is_none());
    }
}
