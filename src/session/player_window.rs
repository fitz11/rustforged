use bevy::camera::visibility::RenderLayers;
use bevy::camera::{RenderTarget, ScalingMode};
use bevy::prelude::*;
#[cfg(target_os = "macos")]
use bevy::window::WindowResolution;
use bevy::window::{WindowCloseRequested, WindowMode, WindowRef};

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
) {
    // Only create if session just became active and no player window exists
    if !session_state.is_active || !existing_windows.is_empty() {
        return;
    }

    let Some(ref monitor_info) = session_state.selected_monitor else {
        return;
    };

    // Spawn the player window with platform-specific strategy
    let window = create_platform_window(monitor_info);
    commands.spawn((window, PlayerWindow));

    info!(
        "Created player window for monitor: {} ({}x{})",
        monitor_info.name, monitor_info.physical_size.x, monitor_info.physical_size.y
    );
}

/// Create the player window with platform-specific strategy.
///
/// On macOS, `BorderlessFullscreen` causes the OS to create a new Space (virtual desktop),
/// which pulls the main editor window to the external monitor. Instead, we create a regular
/// borderless window manually positioned and sized to cover the target monitor.
///
/// winit on macOS has a coordinate mismatch: `MonitorHandle::position()` encodes the logical
/// position using the monitor's own scale factor, but `set_outer_position()` decodes it using
/// the window's scale factor (typically the primary monitor's). We compensate by adjusting:
///   adjusted_position = physical_position * primary_scale / monitor_scale
///
/// On other platforms, `BorderlessFullscreen` works correctly.
fn create_platform_window(monitor_info: &super::state::MonitorInfo) -> Window {
    #[cfg(target_os = "macos")]
    {
        let primary_scale = monitor_info.primary_scale_factor;
        let monitor_scale = monitor_info.scale_factor;

        // Adjust position to compensate for winit's macOS scale factor mismatch.
        // The physical_position was encoded with the monitor's scale, but winit will
        // decode it with the primary monitor's scale, so we pre-correct.
        let adjusted_x =
            (monitor_info.physical_position.x as f64 * primary_scale / monitor_scale) as i32;
        let adjusted_y =
            (monitor_info.physical_position.y as f64 * primary_scale / monitor_scale) as i32;
        let position = bevy::window::WindowPosition::At(IVec2::new(adjusted_x, adjusted_y));

        // Use the target monitor's logical (point) dimensions for the window size.
        // We must NOT use scale_factor_override here: with an override, bevy_winit passes
        // a PhysicalSize to winit, which macOS interprets using the primary monitor's
        // scale factor (since the window starts there before being repositioned). This
        // causes the same coordinate mismatch as the position bug. Without an override,
        // bevy_winit passes a LogicalSize, which macOS interprets as points consistently
        // regardless of which monitor the window initially appears on. Bevy will auto-detect
        // the correct scale factor once the window lands on the target monitor.
        let logical_w = (monitor_info.physical_size.x as f64 / monitor_scale) as u32;
        let logical_h = (monitor_info.physical_size.y as f64 / monitor_scale) as u32;

        Window {
            title: "Player View".into(),
            decorations: false,
            resizable: false,
            mode: WindowMode::Windowed,
            position,
            resolution: WindowResolution::new(logical_w, logical_h),
            window_level: bevy::window::WindowLevel::AlwaysOnTop,
            ..default()
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let mode = if let Some(entity) = monitor_info.entity {
            WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Entity(entity))
        } else {
            WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary)
        };
        Window {
            title: "Player View".into(),
            decorations: false,
            mode,
            ..default()
        }
    }
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
                order: 1,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(window_entity)),
            Projection::Orthographic(OrthographicProjection {
                // AutoMax keeps the projection aspect equal to the window's aspect,
                // so world units map to pixels uniformly on both axes and grid cells
                // stay square in every orientation. (ScalingMode::Fixed stretches to
                // fill and would skew cells whenever viewport_size's aspect differed
                // from the real window framebuffer.)
                scaling_mode: ScalingMode::AutoMax {
                    max_width: viewport_size.x,
                    max_height: viewport_size.y,
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

        // Keep the projection aspect-preserving so grid cells render square in
        // every orientation. AutoMax bounds the visible region to the viewport
        // rectangle while deriving the projection aspect from the window itself,
        // giving a uniform world-to-pixel scale on both axes. This is what keeps
        // squares square regardless of rotation or any mismatch between the
        // monitor's reported aspect and the actual window framebuffer; the earlier
        // ScalingMode::Fixed relied on those aspects matching exactly and skewed
        // cells (and stretched by ~aspect^2 when fed rotation-swapped dimensions).
        let size = session_state.viewport_size;
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scaling_mode = ScalingMode::AutoMax {
                max_width: size.x,
                max_height: size.y,
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
