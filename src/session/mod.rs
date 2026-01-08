mod player_window;
pub mod state;
mod viewport;

pub use state::{LiveSessionState, MonitorInfo, MonitorSelectionDialog, ViewportDragMode, ViewportDragState};
pub use viewport::get_handle_at_position;

use bevy::prelude::*;

pub struct LiveSessionPlugin;

impl Plugin for LiveSessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LiveSessionState>()
            .init_resource::<ViewportDragState>()
            .init_resource::<MonitorSelectionDialog>()
            .init_gizmo_group::<viewport::ViewportGizmoGroup>()
            .add_systems(Startup, viewport::configure_viewport_gizmos)
            .add_systems(
                Update,
                (
                    viewport::draw_viewport_indicator,
                    viewport::handle_viewport_interaction,
                    player_window::create_player_window,
                    player_window::setup_player_camera,
                    player_window::sync_player_camera,
                    player_window::handle_player_window_close,
                ),
            );
    }
}
