//! The editor camera: panning, zooming and framing the map.
//!
//! Pan by dragging with the middle mouse button, or the left one while holding Space, or
//! with the arrow keys, or by scrolling on a trackpad. The scroll wheel (Ctrl+scroll or a
//! pinch on a trackpad) zooms toward the cursor. Home frames the map, or the player view
//! during a session.

use bevy::camera::visibility::RenderLayers;
use bevy::input::gestures::PinchGesture;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use bevy_egui::EguiContexts;

use super::params::{MapExtent, is_cursor_over_ui};
use crate::input::{command_held, shift_held};
use crate::map::render_layer;
use crate::session::{LiveSessionState, PlayerViewport};
use crate::ui::DialogState;

/// How far the view can zoom out (world units per screen pixel) and in
const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.1..=10.0;
/// How much one notch of the scroll wheel zooms
const ZOOM_STEP: f32 = 1.15;
/// Arrow-key panning speed, in screen pixels per second (three times as fast with Shift)
const KEY_PAN_SPEED: f32 = 800.0;

#[derive(Component)]
pub struct EditorCamera;

#[derive(Component)]
pub struct CameraZoom {
    pub scale: f32,
}

impl Default for CameraZoom {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

/// Panning with the mouse
#[derive(Resource, Default)]
pub struct CameraPan {
    /// Space is held, so the left button pans instead of using the tool. Lasts until both
    /// Space and the button are released, so the tool never sees half a click.
    pub space_held: bool,
    /// Where the cursor was last frame during a pan drag
    drag_from: Option<Vec2>,
}

impl CameraPan {
    pub fn is_dragging(&self) -> bool {
        self.drag_from.is_some()
    }
}

/// Run condition: the left mouse button belongs to the tools, not to Space-panning.
pub fn not_space_panning(pan: Res<CameraPan>) -> bool {
    !pan.space_held
}

/// Mouse input that comes before the tools: moving the camera, and pinging. The tools skip
/// a click one of these takes.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreToolInput;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        EditorCamera,
        CameraZoom::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 1000.0)),
        RenderLayers::from_layers(&[render_layer::SHARED, render_layer::EDITOR]),
    ));
}

/// Space starts Space-panning, unless a button is already down (a tool is mid-drag) or a
/// text field or dialog has the keyboard.
pub fn update_space_pan(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    dialog_state: Res<DialogState>,
    mut pan: ResMut<CameraPan>,
) {
    let space = keyboard.pressed(KeyCode::Space);
    let left = mouse_button.pressed(MouseButton::Left);
    let held = if pan.space_held {
        space || left
    } else {
        space
            && !left
            && !mouse_button.just_released(MouseButton::Left)
            && !dialog_state.any_modal_open
            && !dialog_state.text_input_focused
    };
    if pan.space_held != held {
        pan.space_held = held;
    }
}

/// Drags the map with the middle mouse button, or the left one while Space-panning. The map
/// stays under the cursor. A drag has to start on the map, but carries on over the panels.
pub fn camera_pan(
    mouse_button: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut camera_query: Query<(&mut Transform, &CameraZoom), With<EditorCamera>>,
    mut pan: ResMut<CameraPan>,
    mut contexts: EguiContexts,
) {
    let button = if pan.space_held {
        MouseButton::Left
    } else {
        MouseButton::Middle
    };
    let cursor = window.single().ok().and_then(|w| w.cursor_position());

    if !mouse_button.pressed(button) {
        if pan.drag_from.is_some() {
            pan.drag_from = None;
        }
        return;
    }
    if pan.drag_from.is_none() {
        if !mouse_button.just_pressed(button) || is_cursor_over_ui(&mut contexts) {
            return;
        }
        pan.drag_from = cursor;
        return;
    }

    let (Some(from), Some(to)) = (pan.drag_from, cursor) else {
        return;
    };
    pan.drag_from = Some(to);
    let Ok((mut transform, zoom)) = camera_query.single_mut() else {
        return;
    };
    let delta = (to - from) * zoom.scale;
    transform.translation.x -= delta.x;
    transform.translation.y += delta.y;
}

/// Pans the view with the arrow keys (faster with Shift).
pub fn keyboard_pan(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_query: Query<(&mut Transform, &CameraZoom), With<EditorCamera>>,
) {
    let direction = [
        (KeyCode::ArrowLeft, Vec2::NEG_X),
        (KeyCode::ArrowRight, Vec2::X),
        (KeyCode::ArrowUp, Vec2::Y),
        (KeyCode::ArrowDown, Vec2::NEG_Y),
    ]
    .into_iter()
    .filter(|(key, _)| keyboard.pressed(*key))
    .map(|(_, direction)| direction)
    .sum::<Vec2>();
    // Ctrl+arrows are left for other uses
    if direction == Vec2::ZERO || command_held(&keyboard) {
        return;
    }
    let Ok((mut transform, zoom)) = camera_query.single_mut() else {
        return;
    };
    let speed = if shift_held(&keyboard) {
        KEY_PAN_SPEED * 3.0
    } else {
        KEY_PAN_SPEED
    };
    let step = direction.normalize() * speed * zoom.scale * time.delta_secs();
    transform.translation += step.extend(0.0);
}

/// The scroll wheel zooms toward the cursor. On a trackpad, scrolling pans and pinching
/// (or Ctrl+scrolling) zooms.
pub fn camera_zoom(
    mut scroll_events: MessageReader<MouseWheel>,
    mut pinch_events: MessageReader<PinchGesture>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut camera_query: Query<(&mut Transform, &mut CameraZoom), With<EditorCamera>>,
    mut contexts: EguiContexts,
) {
    if is_cursor_over_ui(&mut contexts) {
        scroll_events.clear();
        pinch_events.clear();
        return;
    }
    let Ok((mut transform, mut zoom)) = camera_query.single_mut() else {
        return;
    };
    // The cursor, relative to the middle of the window, in world directions (y up)
    let anchor = window.single().ok().and_then(|window| {
        let offset = window.cursor_position()? - window.size() / 2.0;
        Some(Vec2::new(offset.x, -offset.y))
    });
    let mut camera = Vec2::new(transform.translation.x, transform.translation.y);
    let mut scale = zoom.scale;

    for event in scroll_events.read() {
        match event.unit {
            MouseScrollUnit::Line => {
                (camera, scale) = zoom_about(camera, scale, ZOOM_STEP.powf(-event.y), anchor);
            }
            MouseScrollUnit::Pixel if command_held(&keyboard) => {
                let factor = ZOOM_STEP.powf(-event.y / 50.0);
                (camera, scale) = zoom_about(camera, scale, factor, anchor);
            }
            // Positive deltas move the content right and down
            MouseScrollUnit::Pixel => camera += Vec2::new(-event.x, event.y) * scale,
        }
    }
    for pinch in pinch_events.read() {
        // Positive deltas magnify
        let factor = 1.0 / (1.0 + pinch.0).max(0.1);
        (camera, scale) = zoom_about(camera, scale, factor, anchor);
    }

    transform.translation.x = camera.x;
    transform.translation.y = camera.y;
    if zoom.scale != scale {
        zoom.scale = scale;
    }
}

/// Zooms by `factor` (above 1 zooms out) keeping the world point at `anchor` (an offset
/// from the middle of the screen, in screen pixels) where it is. Returns the new camera
/// position and scale.
fn zoom_about(camera: Vec2, scale: f32, factor: f32, anchor: Option<Vec2>) -> (Vec2, f32) {
    let new_scale = (scale * factor).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
    let Some(anchor) = anchor else {
        return (camera, new_scale);
    };
    let fixed = camera + anchor * scale;
    (fixed - anchor * new_scale, new_scale)
}

/// Home: frames the player view during a session, otherwise everything on the map.
pub fn frame_view(
    session_state: Res<LiveSessionState>,
    map_extent: Res<MapExtent>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut camera_query: Query<(&mut Transform, &mut CameraZoom), With<EditorCamera>>,
    mut contexts: EguiContexts,
) {
    let target = if session_state.is_active {
        Some(PlayerViewport::of(&session_state).world_bounds())
    } else {
        map_extent.0
    };
    let Some(target) = target else {
        return;
    };
    let (Ok(window), Ok((mut transform, mut zoom))) = (window.single(), camera_query.single_mut())
    else {
        return;
    };
    // The part of the window the panels leave for the map
    let window_size = window.size();
    let canvas = contexts
        .ctx_mut()
        .ok()
        .and_then(|ctx| crate::ui::panel_layout::canvas_rect(ctx))
        .map(|rect| {
            (
                Vec2::new(rect.min.x, rect.min.y),
                Vec2::new(rect.max.x, rect.max.y),
            )
        })
        .unwrap_or((Vec2::ZERO, window_size));

    let (camera, scale) = frame(target, canvas, window_size);
    transform.translation.x = camera.x;
    transform.translation.y = camera.y;
    zoom.scale = scale;
}

/// The camera position and scale that show the world rectangle `target` (min, max) in the
/// middle of `canvas` (min, max in screen pixels, y down) on a `window_size` window, with a
/// small margin.
fn frame(target: (Vec2, Vec2), canvas: (Vec2, Vec2), window_size: Vec2) -> (Vec2, f32) {
    const MARGIN: f32 = 0.9;
    let canvas_size = (canvas.1 - canvas.0).max(Vec2::ONE);
    let target_size = (target.1 - target.0).max(Vec2::ONE);
    let scale = (target_size / (canvas_size * MARGIN))
        .max_element()
        .clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
    // Put the target's middle at the canvas's middle
    let offset = (canvas.0 + canvas.1) / 2.0 - window_size / 2.0;
    let offset = Vec2::new(offset.x, -offset.y);
    let target_center = (target.0 + target.1) / 2.0;
    (target_center - offset * scale, scale)
}

/// Run condition: the mouse is panning, or will pan when the left button goes down.
pub fn is_panning(pan: Res<CameraPan>) -> bool {
    pan.space_held || pan.is_dragging()
}

/// A hand cursor while panning, over the tools' own cursors.
pub fn pan_cursor(
    pan: Res<CameraPan>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let icon = if pan.is_dragging() {
        SystemCursorIcon::Grabbing
    } else {
        SystemCursorIcon::Grab
    };
    commands.entity(window).insert(CursorIcon::System(icon));
}

#[allow(clippy::type_complexity)]
pub fn apply_camera_zoom(
    mut camera_query: Query<
        (&CameraZoom, &mut Projection),
        (With<EditorCamera>, Changed<CameraZoom>),
    >,
) {
    for (zoom, mut projection) in camera_query.iter_mut() {
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = zoom.scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_keeps_the_point_under_the_cursor() {
        let camera = Vec2::new(100.0, 50.0);
        let anchor = Vec2::new(200.0, -80.0);
        let under_cursor = camera + anchor * 2.0;

        let (camera, scale) = zoom_about(camera, 2.0, 0.5, Some(anchor));

        assert_eq!(scale, 1.0);
        assert_eq!(camera + anchor * scale, under_cursor);
    }

    #[test]
    fn test_zoom_is_clamped() {
        let (_, scale) = zoom_about(Vec2::ZERO, 9.0, 2.0, None);
        assert_eq!(scale, *ZOOM_RANGE.end());
    }

    #[test]
    fn test_frame_fits_the_target_in_the_canvas() {
        // A 1000x800 window with a 200px panel on the left: the canvas is 800x800
        let canvas = (Vec2::new(200.0, 0.0), Vec2::new(1000.0, 800.0));
        let target = (Vec2::new(0.0, 0.0), Vec2::new(1440.0, 720.0));

        let (camera, scale) = frame(target, canvas, Vec2::new(1000.0, 800.0));

        // Width decides: 1440 world units in 90% of 800 pixels
        assert_eq!(scale, 2.0);
        // The canvas middle is 100px right of the window middle, so the camera is that
        // far (in world units) left of the target middle
        assert_eq!(camera, Vec2::new(720.0 - 200.0, 360.0));
    }
}
