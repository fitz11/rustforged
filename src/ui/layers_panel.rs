use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::map::{FogOfWarData, Layer, MapData, MapDirtyState, PlacedItem, Selected};
use crate::session::LiveSessionState;

/// Resource to track whether the help window is open
#[derive(Resource, Default)]
pub struct HelpWindowState {
    pub is_open: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn layers_panel_ui(
    mut contexts: EguiContexts,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut selected_query: Query<
        (Entity, &mut PlacedItem, &mut Transform, &Sprite, &mut RenderLayers),
        With<Selected>,
    >,
    images: Res<Assets<Image>>,
    mut session_state: ResMut<LiveSessionState>,
    mut help_state: ResMut<HelpWindowState>,
) -> Result {
    egui::SidePanel::right("layers_panel")
        .default_width(200.0)
        .show(contexts.ctx_mut()?, |ui| {
            // =========================================
            // LAYERS SECTION
            // =========================================
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Layers").heading().size(18.0));
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            for layer in Layer::all().iter().rev() {
                if let Some(layer_data) = map_data
                    .layers
                    .iter_mut()
                    .find(|l| l.layer_type == *layer)
                {
                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(4, 4))
                        .show(ui, |ui| {
                            let is_available = layer.is_available();

                            ui.add_enabled_ui(is_available, |ui| {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut layer_data.visible, "");
                                    ui.label(egui::RichText::new(layer.display_name()).size(14.0));

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if !is_available {
                                                ui.label(
                                                    egui::RichText::new("Soon")
                                                        .size(10.0)
                                                        .weak()
                                                        .italics(),
                                                );
                                            } else {
                                                let lock_text =
                                                    if layer_data.locked { "🔒" } else { "🔓" };
                                                if ui
                                                    .button(egui::RichText::new(lock_text).size(14.0))
                                                    .clicked()
                                                {
                                                    layer_data.locked = !layer_data.locked;
                                                }
                                            }
                                        },
                                    );
                                });
                            });
                        });
                }
            }

            // =========================================
            // FOG OF WAR CONTROLS
            // =========================================
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Fog of War").size(14.0).strong());
                let revealed = fog_data.revealed_count();
                let status = if revealed == 0 {
                    "fully fogged".to_string()
                } else {
                    format!("{} revealed", revealed)
                };
                ui.label(egui::RichText::new(status).size(12.0).weak());
            });
            ui.add_space(4.0);

            // Enable/Disable toggle for fog layer
            let mut fog_enabled = map_data
                .layers
                .iter()
                .find(|l| l.layer_type == Layer::FogOfWar)
                .map(|l| l.visible)
                .unwrap_or(true);

            if ui
                .checkbox(&mut fog_enabled, "Enable Fog of War")
                .on_hover_text("Toggle fog visibility for players")
                .changed()
            {
                if let Some(layer_data) = map_data
                    .layers
                    .iter_mut()
                    .find(|l| l.layer_type == Layer::FogOfWar)
                {
                    layer_data.visible = fog_enabled;
                }
                dirty_state.is_dirty = true;
            }

            ui.add_space(4.0);

            // Reset Fog button - simply clears all revealed cells
            let reset_enabled = fog_data.has_revealed_cells();
            if ui
                .add_enabled(
                    reset_enabled,
                    egui::Button::new("Reset Fog").min_size(egui::vec2(160.0, 24.0)),
                )
                .on_hover_text("Hide all revealed areas (cover everything with fog)")
                .clicked()
            {
                fog_data.reset();
                dirty_state.is_dirty = true;
            }

            ui.add_space(12.0);
            ui.separator();

            // =========================================
            // PROPERTIES SECTION
            // =========================================
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Properties").heading().size(18.0));
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            let selected_count = selected_query.iter().count();

            if selected_count == 0 {
                ui.label(egui::RichText::new("No item selected").size(14.0).weak());
            } else if selected_count > 1 {
                ui.label(egui::RichText::new(format!("{} items selected", selected_count)).size(14.0));
                ui.add_space(8.0);

                // Multi-selection: show fit-to-grid and center-to-grid buttons
                if ui.add_sized([140.0, 26.0], egui::Button::new("Fit to Grid (G)")).clicked() {
                    for (_entity, _item, mut transform, sprite, _render_layers) in
                        selected_query.iter_mut()
                    {
                        let original_size = if let Some(custom_size) = sprite.custom_size {
                            custom_size
                        } else if let Some(image) = images.get(&sprite.image) {
                            image.size().as_vec2()
                        } else {
                            Vec2::splat(64.0)
                        };

                        if original_size.x > 0.0 && original_size.y > 0.0 {
                            let grid_size = map_data.grid_size;
                            let scale_x = grid_size / original_size.x;
                            let scale_y = grid_size / original_size.y;
                            let uniform_scale = scale_x.min(scale_y);
                            transform.scale = Vec3::new(uniform_scale, uniform_scale, 1.0);
                        }
                    }
                }

                ui.add_space(4.0);
                if ui.add_sized([140.0, 26.0], egui::Button::new("Center to Grid (C)")).clicked() {
                    let grid_size = map_data.grid_size;
                    let half = grid_size / 2.0;
                    for (_entity, _item, mut transform, _sprite, _render_layers) in
                        selected_query.iter_mut()
                    {
                        let pos = transform.translation.truncate();
                        let snapped = Vec2::new(
                            (pos.x / grid_size).floor() * grid_size + half,
                            (pos.y / grid_size).floor() * grid_size + half,
                        );
                        transform.translation.x = snapped.x;
                        transform.translation.y = snapped.y;
                    }
                }

                ui.add_space(4.0);
                if ui
                    .add_sized([140.0, 26.0], egui::Button::new("Restore Aspect Ratio (A)"))
                    .clicked()
                {
                    for (_entity, _item, mut transform, _sprite, _render_layers) in
                        selected_query.iter_mut()
                    {
                        let uniform_scale = transform.scale.x.abs().max(transform.scale.y.abs());
                        transform.scale.x = uniform_scale;
                        transform.scale.y = uniform_scale;
                    }
                }
            } else {
                // Single selection - show full properties
                if let Ok((_entity, mut item, mut transform, sprite, mut render_layers)) =
                    selected_query.single_mut()
                {
                    // Asset path (truncated if too long)
                    let asset_name = item
                        .asset_path
                        .split('/')
                        .next_back()
                        .unwrap_or(&item.asset_path);
                    ui.label(egui::RichText::new(format!("Asset: {}", asset_name)).size(13.0));

                    ui.add_space(8.0);

                    // Layer selector
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Layer:").size(14.0));
                        egui::ComboBox::from_id_salt("item_layer")
                            .selected_text(item.layer.display_name())
                            .show_ui(ui, |ui| {
                                for layer in Layer::all() {
                                    // Skip FogOfWar layer (not for placing items)
                                    if *layer == Layer::FogOfWar {
                                        continue;
                                    }
                                    let is_selected = item.layer == *layer;
                                    if ui
                                        .selectable_label(is_selected, layer.display_name())
                                        .clicked()
                                    {
                                        item.layer = *layer;
                                        // Update z position to match new layer
                                        transform.translation.z =
                                            layer.z_base() + item.z_index as f32;
                                        // Update render layer for player visibility
                                        *render_layers = if layer.is_player_visible() {
                                            RenderLayers::layer(0)
                                        } else {
                                            RenderLayers::layer(1)
                                        };
                                    }
                                }
                            });
                    });

                    ui.add_space(4.0);

                    // Z-Index property
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Z-Index:").size(14.0));
                        ui.label(egui::RichText::new(format!("{}", item.z_index)).size(14.0));

                        let max_z = Layer::max_z_index();

                        ui.add_enabled_ui(item.z_index < max_z, |ui| {
                            if ui.small_button("+").clicked() {
                                item.z_index += 1;
                                transform.translation.z =
                                    item.layer.z_base() + item.z_index as f32;
                            }
                        });

                        ui.add_enabled_ui(item.z_index > 0, |ui| {
                            if ui.small_button("-").clicked() {
                                item.z_index -= 1;
                                transform.translation.z =
                                    item.layer.z_base() + item.z_index as f32;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Position
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("X:").size(14.0));
                        ui.add(egui::DragValue::new(&mut transform.translation.x).speed(1.0));
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Y:").size(14.0));
                        ui.add(egui::DragValue::new(&mut transform.translation.y).speed(1.0));
                    });

                    ui.add_space(8.0);

                    // Rotation
                    // EulerRot::ZYX returns (z, y, x) - we want the Z rotation (first component)
                    let (rotation, _, _) = transform.rotation.to_euler(EulerRot::ZYX);
                    let mut rotation_deg = rotation.to_degrees();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Rotation:").size(14.0));
                        if ui
                            .add(egui::DragValue::new(&mut rotation_deg).speed(1.0).suffix("°"))
                            .changed()
                        {
                            transform.rotation = Quat::from_rotation_z(rotation_deg.to_radians());
                        }
                    });

                    ui.add_space(8.0);

                    // Scale
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Scale X:").size(14.0));
                        ui.add(egui::DragValue::new(&mut transform.scale.x).speed(0.01));
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Scale Y:").size(14.0));
                        ui.add(egui::DragValue::new(&mut transform.scale.y).speed(0.01));
                    });

                    ui.add_space(8.0);

                    if ui.add_sized([140.0, 26.0], egui::Button::new("Fit to Grid (G)")).clicked() {
                        let original_size = if let Some(custom_size) = sprite.custom_size {
                            custom_size
                        } else if let Some(image) = images.get(&sprite.image) {
                            image.size().as_vec2()
                        } else {
                            Vec2::splat(64.0)
                        };

                        if original_size.x > 0.0 && original_size.y > 0.0 {
                            let grid_size = map_data.grid_size;
                            let scale_x = grid_size / original_size.x;
                            let scale_y = grid_size / original_size.y;
                            let uniform_scale = scale_x.min(scale_y);
                            transform.scale = Vec3::new(uniform_scale, uniform_scale, 1.0);
                        }
                    }

                    ui.add_space(4.0);
                    if ui.add_sized([140.0, 26.0], egui::Button::new("Center to Grid (C)")).clicked() {
                        let grid_size = map_data.grid_size;
                        let half = grid_size / 2.0;
                        let pos = transform.translation.truncate();
                        let snapped = Vec2::new(
                            (pos.x / grid_size).floor() * grid_size + half,
                            (pos.y / grid_size).floor() * grid_size + half,
                        );
                        transform.translation.x = snapped.x;
                        transform.translation.y = snapped.y;
                    }

                    ui.add_space(4.0);
                    if ui.add_sized([140.0, 26.0], egui::Button::new("Restore Aspect Ratio (A)")).clicked() {
                        let uniform_scale = transform.scale.x.abs().max(transform.scale.y.abs());
                        transform.scale.x = uniform_scale;
                        transform.scale.y = uniform_scale;
                    }
                }
            }

            // =========================================
            // LIVE SESSION SECTION (when active)
            // =========================================
            if session_state.is_active {
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Live Session").heading().size(18.0));
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                if let Some(ref monitor) = session_state.selected_monitor {
                    ui.label(egui::RichText::new(format!("Monitor: {}", monitor.name)).size(13.0));
                    ui.label(egui::RichText::new(format!(
                        "{}x{}",
                        monitor.physical_size.x, monitor.physical_size.y
                    )).size(13.0).weak());
                }

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Position").size(14.0).strong());

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("X:").size(14.0));
                    ui.add(
                        egui::DragValue::new(&mut session_state.viewport_center.x)
                            .speed(5.0)
                            .suffix(" px"),
                    );
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Y:").size(14.0));
                    ui.add(
                        egui::DragValue::new(&mut session_state.viewport_center.y)
                            .speed(5.0)
                            .suffix(" px"),
                    );
                });

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Size").size(14.0).strong());

                // Width can be edited, height is locked to aspect ratio
                let aspect = session_state.monitor_aspect_ratio();
                let mut width = session_state.viewport_size.x;

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("W:").size(14.0));
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
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("H:").size(14.0));
                    ui.label(egui::RichText::new(format!("{:.0} px", session_state.viewport_size.y)).size(14.0));
                });

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Rotation").size(14.0).strong());

                ui.horizontal(|ui| {
                    if ui.add_sized([28.0, 26.0], egui::Button::new("↺")).clicked() {
                        session_state.rotate_ccw();
                    }
                    ui.label(egui::RichText::new(format!("{}°", session_state.rotation_degrees)).size(14.0));
                    if ui.add_sized([28.0, 26.0], egui::Button::new("↻")).clicked() {
                        session_state.rotate_cw();
                    }
                });

                ui.add_space(12.0);

                if ui.add_sized([140.0, 26.0], egui::Button::new("End Session")).clicked() {
                    session_state.is_active = false;
                }
            }

            // =========================================
            // HELP BUTTON (bottom center)
            // =========================================
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if ui
                    .add_sized([120.0, 28.0], egui::Button::new("Help (H)"))
                    .clicked()
                {
                    help_state.is_open = true;
                }
            });
        });
    Ok(())
}

/// Renders the help popup window with keyboard shortcuts and usage instructions
pub fn help_popup_ui(mut contexts: EguiContexts, mut help_state: ResMut<HelpWindowState>) -> Result {
    if !help_state.is_open {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Help")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(400.0)
        .show(ctx, |ui| {
            ui.heading("Rustforged - D&D 5E VTT Map Editor");
            ui.separator();

            // Tools Section
            ui.heading("Tools");
            egui::Grid::new("tools_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("V / S");
                    ui.label("Select - Click to select items, drag to move");
                    ui.end_row();

                    ui.strong("P");
                    ui.label("Place - Single-click to place selected asset");
                    ui.end_row();

                    ui.strong("B");
                    ui.label("Brush - Drag to continuously place assets");
                    ui.end_row();

                    ui.strong("D");
                    ui.label("Draw - Freehand annotation paths");
                    ui.end_row();

                    ui.strong("L");
                    ui.label("Line - Straight line annotations");
                    ui.end_row();

                    ui.strong("T");
                    ui.label("Text - Click to add text annotations");
                    ui.end_row();

                    ui.strong("F");
                    ui.label("Fog - Reveal/hide fog of war areas");
                    ui.end_row();

                    ui.strong("C / Shift+C");
                    ui.label("Cycle layer (Place/Brush tools)");
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();

            // Selection Shortcuts
            ui.heading("Selection");
            egui::Grid::new("selection_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Click");
                    ui.label("Select topmost item");
                    ui.end_row();

                    ui.strong("Ctrl+Click");
                    ui.label("Toggle selection (multi-select)");
                    ui.end_row();

                    ui.strong("Drag (empty)");
                    ui.label("Box selection");
                    ui.end_row();

                    ui.strong("Escape");
                    ui.label("Clear selection");
                    ui.end_row();

                    ui.strong("Delete / Backspace");
                    ui.label("Delete selected items");
                    ui.end_row();

                    ui.strong("G");
                    ui.label("Fit selected to grid cell");
                    ui.end_row();

                    ui.strong("C");
                    ui.label("Center selected to grid");
                    ui.end_row();

                    ui.strong("A");
                    ui.label("Restore aspect ratio");
                    ui.end_row();

                    ui.strong("R / Shift+R");
                    ui.label("Rotate 90° CW / CCW");
                    ui.end_row();

                    ui.strong("Ctrl+C / Ctrl+X");
                    ui.label("Copy / Cut selected items");
                    ui.end_row();

                    ui.strong("Ctrl+V");
                    ui.label("Paste items");
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();

            // Camera Controls
            ui.heading("Camera");
            egui::Grid::new("camera_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Middle Mouse Drag");
                    ui.label("Pan camera");
                    ui.end_row();

                    ui.strong("Scroll Wheel");
                    ui.label("Zoom in/out");
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();

            // Placement
            ui.heading("Placement");
            egui::Grid::new("placement_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Click (Place tool)");
                    ui.label("Place asset at grid-snapped position");
                    ui.end_row();

                    ui.strong("Shift+Click");
                    ui.label("Free placement (bypass grid snap)");
                    ui.end_row();

                    ui.strong("Shift+Drag");
                    ui.label("Snap selected to grid while dragging");
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();

            // Asset Management
            ui.heading("Assets");
            egui::Grid::new("asset_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("F2");
                    ui.label("Rename selected asset");
                    ui.end_row();

                    ui.strong("F3");
                    ui.label("Rename current map");
                    ui.end_row();

                    ui.strong("F4");
                    ui.label("Rename library");
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();

            // File Operations
            ui.heading("File");
            egui::Grid::new("file_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.strong("Ctrl+S");
                    ui.label("Save map");
                    ui.end_row();

                    ui.strong("Ctrl+Shift+S");
                    ui.label("Save as...");
                    ui.end_row();

                    ui.strong("Ctrl+O");
                    ui.label("Open map");
                    ui.end_row();

                    ui.strong("Ctrl+N");
                    ui.label("New map");
                    ui.end_row();

                    ui.strong("H");
                    ui.label("Toggle this help window");
                    ui.end_row();
                });

            ui.add_space(15.0);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                if ui.button("Close").clicked() {
                    help_state.is_open = false;
                }
            });
        });

    // Close on Escape key
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        help_state.is_open = false;
    }

    Ok(())
}

/// Handles the H keyboard shortcut to toggle help window
pub fn handle_help_shortcut(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut help_state: ResMut<HelpWindowState>,
    mut contexts: EguiContexts,
) {
    // Don't toggle if typing in a text field
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    if keyboard.just_pressed(KeyCode::KeyH) {
        help_state.is_open = !help_state.is_open;
    }
}
