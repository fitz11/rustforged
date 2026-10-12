pub mod freeze;
mod keep_awake;
mod player_window;
pub mod state;
mod viewport;

pub use freeze::PlayerMapView;
pub use state::{
    LiveSessionState, MonitorInfo, MonitorSelectionDialog, PlayerViewport, ViewportDragMode,
    ViewportDragState,
};

use bevy::prelude::*;

use crate::editor::session_is_active;

pub struct LiveSessionPlugin;

/// Ctrl+B (Cmd+B on macOS): black out the player screen, or show the map again
fn toggle_blackout(mut session_state: ResMut<LiveSessionState>) {
    session_state.blackout = !session_state.blackout;
}

/// Systems that settle what players see this frame: the frozen snapshot and the player
/// camera. Anything drawn for players (fog, grid) runs after them, so it covers the area
/// the camera shows this frame rather than last frame's.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerViewSync;

impl Plugin for LiveSessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LiveSessionState>()
            .init_resource::<ViewportDragState>()
            .init_resource::<MonitorSelectionDialog>()
            .init_resource::<PlayerViewport>()
            .init_resource::<freeze::FrozenPlayerView>()
            .init_resource::<keep_awake::KeepAwakeGuard>()
            .init_gizmo_group::<viewport::ViewportGizmoGroup>()
            .add_systems(Startup, viewport::configure_viewport_gizmos)
            .add_systems(
                Update,
                (
                    keep_awake::sync_keep_awake.run_if(resource_changed::<LiveSessionState>),
                    toggle_blackout
                        .run_if(
                            session_is_active
                                .and_then(crate::editor::command_key(KeyCode::KeyB))
                                .and_then(crate::editor::shortcuts_enabled),
                        )
                        .before(PlayerViewSync),
                    viewport::draw_viewport_indicator.run_if(session_is_active),
                    // Before the tools, so grabbing a viewport handle keeps the click
                    // from also revealing fog, placing an item or selecting
                    viewport::handle_viewport_interaction
                        .run_if(session_is_active.and_then(crate::editor::not_space_panning))
                        .after(crate::editor::CameraInput)
                        .before(crate::editor::ToolInput),
                    player_window::create_player_window
                        .run_if(resource_changed::<LiveSessionState>),
                    player_window::setup_player_camera,
                    player_window::keep_focus_in_editor,
                    // No run_if here - needs to run to unfreeze after session ends
                    freeze::sync_freeze
                        .before(player_window::sync_player_camera)
                        .after(viewport::handle_viewport_interaction)
                        .in_set(PlayerViewSync),
                    player_window::sync_player_camera
                        .run_if(session_is_active)
                        .in_set(PlayerViewSync),
                    // No run_if here - needs to run to clean up after session ends
                    player_window::handle_player_window_close,
                    player_window::handle_player_window_close_request,
                    player_window::handle_graceful_shutdown,
                ),
            );
    }
}
