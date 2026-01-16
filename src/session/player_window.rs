use bevy::camera::visibility::RenderLayers;
use bevy::camera::{RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy::window::{Monitor, WindowCloseRequested, WindowMode, WindowRef};

use super::state::LiveSessionState;

/// Marker component for the player view window
#[derive(Component)]
pub struct PlayerWindow;

/// Marker component for the player camera
#[derive(Component)]
pub struct PlayerCamera;

/// Create the player window when a session becomes active
pub fn create_player_window(
    mut commands: Commands,
    session_state: Res<LiveSessionState>,
    existing_windows: Query<Entity, With<PlayerWindow>>,
    monitors: Query<(Entity, &Monitor)>,
) {
    // Only create if session just became active and no player window exists
    if !session_state.is_active || !existing_windows.is_empty() {
        return;
    }

    let Some(ref monitor_info) = session_state.selected_monitor else {
        return;
    };

    // Find the monitor entity by index
    let monitor_entity = monitors
        .iter()
        .nth(monitor_info.index)
        .map(|(e, _)| e);

    // Spawn the player window
    let mut window = Window {
        title: "Player View".into(),
        decorations: false,
        ..default()
    };

    // Set fullscreen mode with the selected monitor
    if let Some(entity) = monitor_entity {
        window.mode = WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Entity(entity));
    } else {
        // Fallback to primary monitor fullscreen
        window.mode = WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary);
    }

    commands.spawn((window, PlayerWindow));

    info!(
        "Created player window for monitor: {} ({}x{})",
        monitor_info.name, monitor_info.physical_size.x, monitor_info.physical_size.y
    );
}

/// Set up the camera for the player window once it's created
pub fn setup_player_camera(
    mut commands: Commands,
    session_state: Res<LiveSessionState>,
    player_windows: Query<Entity, (With<PlayerWindow>, Added<Window>)>,
    existing_cameras: Query<Entity, With<PlayerCamera>>,
) {
    // Only set up camera if we don't already have one
    if !existing_cameras.is_empty() {
        return;
    }

    for window_entity in player_windows.iter() {
        let viewport_size = session_state.viewport_size;

        commands.spawn((
            Camera2d,
            PlayerCamera,
            Camera {
                target: RenderTarget::Window(WindowRef::Entity(window_entity)),
                order: 1,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: viewport_size.x,
                    height: viewport_size.y,
                },
                near: -1000.0,
                far: 1000.0,
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_translation(session_state.viewport_center.extend(1000.0)),
            // Render layer 0 (main content) and layer 2 (player-only fog)
            // Does not see layer 1 (editor-only: annotations, viewport indicator)
            RenderLayers::from_layers(&[0, 2]),
        ));

        info!("Created player camera targeting window {:?}", window_entity);
    }
}

/// Sync the player camera position, rotation, and projection with the viewport
pub fn sync_player_camera(
    session_state: Res<LiveSessionState>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<PlayerCamera>>,
) {
    if !session_state.is_active {
        return;
    }

    for (mut transform, mut projection) in camera_query.iter_mut() {
        // Update camera position to viewport center
        transform.translation.x = session_state.viewport_center.x;
        transform.translation.y = session_state.viewport_center.y;

        // Apply rotation (negative because camera rotation is inverse of content rotation)
        transform.rotation = Quat::from_rotation_z(-session_state.rotation_radians());

        // Update projection to match viewport size
        // Use effective size which accounts for rotation
        let effective_size = session_state.effective_viewport_size();
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scaling_mode = ScalingMode::Fixed {
                width: effective_size.x,
                height: effective_size.y,
            };
        }
    }
}

/// Handle closing the player window (ESC key or session ending)
pub fn handle_player_window_close(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut session_state: ResMut<LiveSessionState>,
    player_windows: Query<Entity, With<PlayerWindow>>,
    player_cameras: Query<Entity, With<PlayerCamera>>,
    focused_window: Query<&Window, With<PlayerWindow>>,
) {
    // Check if ESC was pressed while player window is focused
    let player_focused = focused_window
        .iter()
        .any(|w| w.focused);

    if player_focused && keyboard.just_pressed(KeyCode::Escape) {
        session_state.is_active = false;
    }

    // Clean up when session is no longer active
    if !session_state.is_active {
        for entity in player_windows.iter() {
            commands.entity(entity).despawn();
        }
        for entity in player_cameras.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Handle graceful shutdown when the application is exiting
///
/// This ensures the player window and camera are properly despawned before
/// the application exits, preventing any potential issues with orphaned windows.
pub fn handle_graceful_shutdown(
    mut commands: Commands,
    mut exit_events: MessageReader<AppExit>,
    mut session_state: ResMut<LiveSessionState>,
    player_windows: Query<Entity, With<PlayerWindow>>,
    player_cameras: Query<Entity, With<PlayerCamera>>,
) {
    for _event in exit_events.read() {
        // Deactivate the session
        if session_state.is_active {
            info!("Application exiting, deactivating live session");
            session_state.is_active = false;
        }

        // Immediately despawn player window and camera to ensure clean exit
        for entity in player_windows.iter() {
            commands.entity(entity).despawn();
        }
        for entity in player_cameras.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Handle close requests specifically for the player window (e.g., Alt+F4 on player window)
///
/// This handles the case where the user closes the player window directly, rather than
/// closing the main application. In this case, we just deactivate the session.
pub fn handle_player_window_close_request(
    mut commands: Commands,
    mut close_events: MessageReader<WindowCloseRequested>,
    mut session_state: ResMut<LiveSessionState>,
    player_windows: Query<Entity, With<PlayerWindow>>,
    player_cameras: Query<Entity, With<PlayerCamera>>,
) {
    let player_window_entity = player_windows.iter().next();

    for event in close_events.read() {
        // Only handle close requests for the player window
        if Some(event.window) == player_window_entity {
            session_state.is_active = false;

            // Despawn the player window and camera
            for entity in player_windows.iter() {
                commands.entity(entity).despawn();
            }
            for entity in player_cameras.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}
