use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::map::{Layer, MapData, PlacedItem, Selected};
use crate::session::LiveSessionState;

pub fn layers_panel_ui(
    mut contexts: EguiContexts,
    mut map_data: ResMut<MapData>,
    mut selected_query: Query<(Entity, &mut PlacedItem, &mut Transform, &Sprite), With<Selected>>,
    images: Res<Assets<Image>>,
    mut session_state: ResMut<LiveSessionState>,
) -> Result {
    egui::SidePanel::right("layers_panel")
        .default_width(200.0)
        .show(contexts.ctx_mut()?, |ui| {
            // =========================================
            // LAYERS SECTION
            // =========================================
            ui.heading("Layers");
            ui.separator();

            for layer in Layer::all().iter().rev() {
                if let Some(layer_data) = map_data
                    .layers
                    .iter_mut()
                    .find(|l| l.layer_type == *layer)
                {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut layer_data.visible, "");
                        ui.label(layer.display_name());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if layer_data.locked {
                                if ui.small_button("🔒").clicked() {
                                    layer_data.locked = false;
                                }
                            } else if ui.small_button("🔓").clicked() {
                                layer_data.locked = true;
                            }
                        });
                    });
                }
            }

            ui.add_space(10.0);
            ui.separator();

            // =========================================
            // PROPERTIES SECTION
            // =========================================
            ui.heading("Properties");
            ui.separator();

            let selected_count = selected_query.iter().count();

            if selected_count == 0 {
                ui.label("No item selected");
            } else if selected_count > 1 {
                ui.label(format!("{} items selected", selected_count));
                ui.add_space(5.0);

                // Multi-selection: only show fit-to-grid button
                if ui.button("Fit to Grid (G)").clicked() {
                    for (_entity, _item, mut transform, sprite) in selected_query.iter_mut() {
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
            } else {
                // Single selection - show full properties
                if let Ok((_entity, mut item, mut transform, sprite)) = selected_query.single_mut() {
                    // Asset path (truncated if too long)
                    let asset_name = item
                        .asset_path
                        .split('/')
                        .next_back()
                        .unwrap_or(&item.asset_path);
                    ui.label(format!("Asset: {}", asset_name));

                    ui.add_space(5.0);

                    // Layer selector
                    ui.horizontal(|ui| {
                        ui.label("Layer:");
                        egui::ComboBox::from_id_salt("item_layer")
                            .selected_text(item.layer.display_name())
                            .show_ui(ui, |ui| {
                                for layer in Layer::all() {
                                    let is_selected = item.layer == *layer;
                                    if ui
                                        .selectable_label(is_selected, layer.display_name())
                                        .clicked()
                                    {
                                        item.layer = *layer;
                                        // Update z position to match new layer
                                        transform.translation.z =
                                            layer.z_base() + item.z_index as f32;
                                    }
                                }
                            });
                    });

                    ui.add_space(5.0);

                    // Position
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        ui.add(egui::DragValue::new(&mut transform.translation.x).speed(1.0));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut transform.translation.y).speed(1.0));
                    });

                    ui.add_space(5.0);

                    // Rotation
                    // EulerRot::ZYX returns (z, y, x) - we want the Z rotation (first component)
                    let (rotation, _, _) = transform.rotation.to_euler(EulerRot::ZYX);
                    let mut rotation_deg = rotation.to_degrees();
                    ui.horizontal(|ui| {
                        ui.label("Rotation:");
                        if ui
                            .add(egui::DragValue::new(&mut rotation_deg).speed(1.0).suffix("°"))
                            .changed()
                        {
                            transform.rotation = Quat::from_rotation_z(rotation_deg.to_radians());
                        }
                    });

                    ui.add_space(5.0);

                    // Scale
                    ui.horizontal(|ui| {
                        ui.label("Scale X:");
                        ui.add(egui::DragValue::new(&mut transform.scale.x).speed(0.01));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Scale Y:");
                        ui.add(egui::DragValue::new(&mut transform.scale.y).speed(0.01));
                    });

                    ui.add_space(5.0);

                    if ui.button("Fit to Grid (G)").clicked() {
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
            }

            // =========================================
            // LIVE SESSION SECTION (when active)
            // =========================================
            if session_state.is_active {
                ui.add_space(10.0);
                ui.separator();
                ui.heading("Live Session");
                ui.separator();

                if let Some(ref monitor) = session_state.selected_monitor {
                    ui.label(format!("Monitor: {}", monitor.name));
                    ui.label(format!(
                        "{}x{}",
                        monitor.physical_size.x, monitor.physical_size.y
                    ));
                }

                ui.add_space(5.0);
                ui.label("Position");

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

                ui.add_space(5.0);
                ui.label("Size");

                // Width can be edited, height is locked to aspect ratio
                let aspect = session_state.monitor_aspect_ratio();
                let mut width = session_state.viewport_size.x;

                ui.horizontal(|ui| {
                    ui.label("W:");
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
                    ui.label("H:");
                    ui.label(format!("{:.0} px", session_state.viewport_size.y));
                });

                ui.add_space(5.0);
                ui.label("Rotation");

                ui.horizontal(|ui| {
                    if ui.button("↺").clicked() {
                        session_state.rotate_ccw();
                    }
                    ui.label(format!("{}°", session_state.rotation_degrees));
                    if ui.button("↻").clicked() {
                        session_state.rotate_cw();
                    }
                });

                ui.add_space(10.0);

                if ui.button("End Session").clicked() {
                    session_state.is_active = false;
                }
            }
        });
    Ok(())
}
