//! Help popup window and keyboard shortcut handling.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::ui::shortcuts::{SHORTCUTS, ShortcutGroup, display_keys};
use crate::update::UpdateState;

use super::HelpWindowState;

/// Renders the help popup window with keyboard shortcuts and usage instructions.
pub fn help_popup_ui(
    mut contexts: EguiContexts,
    mut help_state: ResMut<HelpWindowState>,
    update_state: Res<UpdateState>,
) -> Result {
    if !help_state.is_open {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Help")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(400.0)
        .max_height(800.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Rustforged - D&D 5E VTT Map Editor");

                ui.horizontal(|ui| {
                    ui.label("Version:");
                    ui.strong(crate::update::CURRENT_VERSION);

                    if update_state.update_available
                        && let Some(ref version) = update_state.latest_version
                    {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 165, 0),
                            format!("(v{} available)", version),
                        );
                    }
                });

                ui.separator();

                // Every shortcut, from the one table the README is checked against
                for group in SHORTCUTS {
                    render_shortcut_group(ui, group);
                    ui.add_space(10.0);
                    ui.separator();
                }

                ui.horizontal(|ui| {
                    ui.label("GitHub:");
                    ui.hyperlink_to("fitz11/rustforged", "https://github.com/fitz11/rustforged");
                });

                ui.add_space(10.0);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui.button("Close").clicked() {
                        help_state.is_open = false;
                    }
                });
            });
        });

    // Close on Escape key
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        help_state.is_open = false;
    }

    Ok(())
}

fn render_shortcut_group(ui: &mut egui::Ui, group: &ShortcutGroup) {
    ui.heading(group.title);
    egui::Grid::new(group.title)
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            for shortcut in group.shortcuts {
                ui.strong(display_keys(shortcut.keys));
                ui.label(shortcut.action);
                ui.end_row();
            }
        });
}

/// Handles the H keyboard shortcut to toggle help window.
pub fn handle_help_shortcut(mut help_state: ResMut<HelpWindowState>) {
    help_state.is_open = !help_state.is_open;
}
