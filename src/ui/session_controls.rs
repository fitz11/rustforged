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
            scale_factor: monitor.scale_factor,
            position: monitor.physical_position,
            index,
        });
    }

    // Fallback: try WinitMonitors resource if no Monitor entities found
    if new_monitors.is_empty() {
        if let Some(winit_monitors) = winit_monitors {
            for index in 0..10 {
                if let Some(handle) = winit_monitors.nth(index) {
                    let size = handle.size();
                    let name = handle
                        .name()
                        .unwrap_or_else(|| format!("Monitor {}", index + 1));
                    new_monitors.push(MonitorInfo {
                        name,
                        physical_size: UVec2::new(size.width, size.height),
                        scale_factor: handle.scale_factor(),
                        position: {
                            let pos = handle.position();
                            IVec2::new(pos.x, pos.y)
                        },
                        index,
                    });
                } else {
                    break;
                }
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

/// Render viewport properties panel when session is active
pub fn viewport_properties_ui(
    mut contexts: EguiContexts,
    mut session_state: ResMut<LiveSessionState>,
) -> Result {
    if !session_state.is_active {
        return Ok(());
    }

    egui::Window::new("Player Viewport")
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 60.0])
        .default_width(200.0)
        .show(contexts.ctx_mut()?, |ui| {
            if let Some(ref monitor) = session_state.selected_monitor {
                ui.label(format!("Monitor: {}", monitor.name));
                ui.label(format!(
                    "Resolution: {}x{}",
                    monitor.physical_size.x, monitor.physical_size.y
                ));
                let aspect = monitor.aspect_ratio();
                ui.label(format!("Aspect Ratio: {:.2}", aspect));
            }

            ui.separator();
            ui.heading("Position");

            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(
                    egui::DragValue::new(&mut session_state.viewport_center.x)
                        .speed(5.0)
                        .suffix(" px"),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Y:");
                ui.add(
                    egui::DragValue::new(&mut session_state.viewport_center.y)
                        .speed(5.0)
                        .suffix(" px"),
                );
            });

            ui.separator();
            ui.heading("Size");

            // Width can be edited, height is locked to aspect ratio
            let aspect = session_state.monitor_aspect_ratio();
            let mut width = session_state.viewport_size.x;

            ui.horizontal(|ui| {
                ui.label("Width:");
                if ui
                    .add(
                        egui::DragValue::new(&mut width)
                            .speed(5.0)
                            .range(100.0..=10000.0)
                            .suffix(" px"),
                    )
                    .changed()
                {
                    session_state.viewport_size.x = width;
                    session_state.viewport_size.y = width / aspect;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Height:");
                ui.label(format!("{:.0} px (locked)", session_state.viewport_size.y));
            });

            ui.separator();
            ui.heading("Rotation");

            ui.horizontal(|ui| {
                if ui.button("\u{21BA} CCW").clicked() {
                    session_state.rotate_ccw();
                }
                ui.label(format!("{}°", session_state.rotation_degrees));
                if ui.button("CW \u{21BB}").clicked() {
                    session_state.rotate_cw();
                }
            });

            ui.separator();

            if ui.button("End Session").clicked() {
                session_state.is_active = false;
            }
        });

    Ok(())
}
