use bevy::prelude::*;
use bevy::window::Monitor;
use bevy::winit::WinitMonitors;
use bevy_egui::{egui, EguiContexts};

use crate::session::{LiveSessionState, MonitorInfo, MonitorSelectionDialog};

/// Enumerate available monitors using Bevy's Monitor entities
/// This runs every frame when the dialog is open to handle late monitor discovery
pub fn enumerate_monitors(
    winit_monitors: Option<Res<WinitMonitors>>,
    monitors_query: Query<(Entity, &Monitor)>,
    mut dialog: ResMut<MonitorSelectionDialog>,
) {
    // Only enumerate when dialog is open
    if !dialog.is_open {
        return;
    }

    // Re-enumerate each frame in case monitors weren't ready initially
    let mut new_monitors = Vec::new();

    // First try to use the Monitor entities (Bevy's preferred approach)
    for (index, (_entity, monitor)) in monitors_query.iter().enumerate() {
        new_monitors.push(MonitorInfo {
            name: monitor
                .name
                .clone()
                .unwrap_or_else(|| format!("Monitor {}", index + 1)),
            physical_size: UVec2::new(monitor.physical_width, monitor.physical_height),
            physical_position: monitor.physical_position,
            scale_factor: monitor.scale_factor,
            index,
        });
    }

    // Fallback: try WinitMonitors resource if no Monitor entities found
    if new_monitors.is_empty()
        && let Some(winit_monitors) = winit_monitors
    {
        for index in 0..10 {
            if let Some(handle) = winit_monitors.nth(index) {
                let size = handle.size();
                let name = handle
                    .name()
                    .unwrap_or_else(|| format!("Monitor {}", index + 1));
                let position = handle.position();
                new_monitors.push(MonitorInfo {
                    name,
                    physical_size: UVec2::new(size.width, size.height),
                    physical_position: IVec2::new(position.x, position.y),
                    scale_factor: handle.scale_factor(),
                    index,
                });
            } else {
                break;
            }
        }
    }

    // Only log and update if the count changed
    if new_monitors.len() != dialog.available_monitors.len() {
        if new_monitors.is_empty() {
            warn!("No monitors found");
        } else {
            info!("Found {} monitors", new_monitors.len());
        }
    }

    dialog.available_monitors = new_monitors;
}

/// Render the monitor selection dialog
pub fn monitor_selection_dialog(
    mut contexts: EguiContexts,
    mut dialog: ResMut<MonitorSelectionDialog>,
    mut session_state: ResMut<LiveSessionState>,
) -> Result {
    if !dialog.is_open {
        return Ok(());
    }

    let mut close_dialog = false;

    egui::Window::new("Start Live Session")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .show(contexts.ctx_mut()?, |ui| {
            ui.label("Select the monitor for the player view:");
            ui.separator();

            if dialog.available_monitors.is_empty() {
                ui.label("Scanning for monitors...");
            } else {
                for monitor in &dialog.available_monitors {
                    let text = format!(
                        "{}: {}x{}",
                        monitor.name, monitor.physical_size.x, monitor.physical_size.y
                    );

                    if ui.button(&text).clicked() {
                        // Initialize session with this monitor
                        let aspect = monitor.aspect_ratio();
                        session_state.selected_monitor = Some(monitor.clone());
                        session_state.viewport_size = Vec2::new(700.0, 700.0 / aspect);
                        session_state.viewport_center = Vec2::ZERO;
                        session_state.is_active = true;
                        close_dialog = true;

                        info!("Starting live session on monitor: {}", monitor.name);
                    }
                }
            }

            ui.separator();
            if ui.button("Cancel").clicked() {
                close_dialog = true;
            }
        });

    if close_dialog {
        dialog.is_open = false;
        dialog.available_monitors.clear();
    }

    Ok(())
}

