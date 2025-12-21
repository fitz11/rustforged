use bevy::camera::visibility::RenderLayers;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

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

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        EditorCamera,
        CameraZoom::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 1000.0)),
        // Layer 0 = main content, Layer 1 = editor-only (viewport indicator)
        RenderLayers::from_layers(&[0, 1]),
    ));
}

pub fn camera_pan(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut camera_query: Query<(&mut Transform, &CameraZoom), With<EditorCamera>>,
) {
    if !mouse_button.pressed(MouseButton::Middle) {
        mouse_motion.clear();
        return;
    }

    let Ok((mut transform, zoom)) = camera_query.single_mut() else {
        return;
    };

    for event in mouse_motion.read() {
        let delta = event.delta * zoom.scale;
        transform.translation.x -= delta.x;
        transform.translation.y += delta.y;
    }
}

pub fn camera_zoom(
    mut scroll_events: MessageReader<MouseWheel>,
    mut camera_query: Query<&mut CameraZoom, With<EditorCamera>>,
) {
    let Ok(mut zoom) = camera_query.single_mut() else {
        return;
    };

    for event in scroll_events.read() {
        let scroll_amount = match event.unit {
            MouseScrollUnit::Line => event.y * 0.1,
            MouseScrollUnit::Pixel => event.y * 0.001,
        };

        zoom.scale = (zoom.scale - scroll_amount).clamp(0.1, 10.0);
    }
}

pub fn apply_camera_zoom(
    mut camera_query: Query<(&CameraZoom, &mut Projection), (With<EditorCamera>, Changed<CameraZoom>)>,
) {
    for (zoom, mut projection) in camera_query.iter_mut() {
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = zoom.scale;
        }
    }
}
