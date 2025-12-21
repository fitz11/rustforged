use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::map::{Layer, MapData, PlacedItem, Selected};

pub fn properties_panel_ui(
    mut contexts: EguiContexts,
    mut selected_query: Query<(Entity, &mut PlacedItem, &mut Transform, &Sprite), With<Selected>>,
    map_data: Res<MapData>,
    images: Res<Assets<Image>>,
) -> Result {
    egui::Window::new("Properties")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .resizable(false)
        .show(contexts.ctx_mut()?, |ui| {
            let selected_count = selected_query.iter().count();

            if selected_count == 0 {
                ui.label("No item selected");
                return;
            }

            if selected_count > 1 {
                ui.label(format!("{} items selected", selected_count));
                ui.separator();

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
                return;
            }

            // Single selection - show full properties
            if let Ok((_entity, mut item, mut transform, sprite)) = selected_query.single_mut() {
                ui.label(format!("Asset: {}", item.asset_path));

                ui.separator();

                // Layer selector
                ui.horizontal(|ui| {
                    ui.label("Layer:");
                    egui::ComboBox::from_id_salt("item_layer")
                        .selected_text(item.layer.display_name())
                        .show_ui(ui, |ui| {
                            for layer in Layer::all() {
                                let is_selected = item.layer == *layer;
                                if ui.selectable_label(is_selected, layer.display_name()).clicked() {
                                    item.layer = *layer;
                                    // Update z position to match new layer
                                    transform.translation.z = layer.z_base() + item.z_index as f32;
                                }
                            }
                        });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut transform.translation.x).speed(1.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut transform.translation.y).speed(1.0));
                });

                ui.separator();

                let (_, rotation, _) = transform.rotation.to_euler(EulerRot::ZYX);
                let mut rotation_deg = rotation.to_degrees();
                ui.horizontal(|ui| {
                    ui.label("Rotation:");
                    if ui
                        .add(egui::DragValue::new(&mut rotation_deg).speed(1.0))
                        .changed()
                    {
                        transform.rotation = Quat::from_rotation_z(rotation_deg.to_radians());
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Scale X:");
                    ui.add(egui::DragValue::new(&mut transform.scale.x).speed(0.01));
                });

                ui.horizontal(|ui| {
                    ui.label("Scale Y:");
                    ui.add(egui::DragValue::new(&mut transform.scale.y).speed(0.01));
                });

                ui.separator();

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
        });
    Ok(())
}
