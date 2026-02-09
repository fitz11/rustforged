//! Properties panel UI for selected items.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::map::{Layer, MapData, PlacedItem, Selected};

/// Selected item query type for the properties panel.
pub type SelectedQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut PlacedItem,
        &'static mut Transform,
        &'static Sprite,
        &'static mut RenderLayers,
    ),
    With<Selected>,
>;

/// Renders the properties section for selected items.
pub fn render_properties(
    ui: &mut egui::Ui,
    map_data: &MapData,
    selected_query: &mut SelectedQuery,
    images: &Assets<Image>,
) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Properties").heading().size(18.0));
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    let selected_count = selected_query.iter().count();

    if selected_count == 0 {
        ui.label(egui::RichText::new("No item selected").size(14.0).weak());
    } else if selected_count > 1 {
        render_multi_selection(ui, map_data, selected_query, images);
    } else {
        render_single_selection(ui, map_data, selected_query, images);
    }
}

/// Renders UI for multiple selected items.
fn render_multi_selection(
    ui: &mut egui::Ui,
    map_data: &MapData,
    selected_query: &mut SelectedQuery,
    images: &Assets<Image>,
) {
    let selected_count = selected_query.iter().count();
    ui.label(egui::RichText::new(format!("{} items selected", selected_count)).size(14.0));
    ui.add_space(8.0);

    // Multi-selection: show fit-to-grid and center-to-grid buttons
    if ui
        .add_sized([140.0, 26.0], egui::Button::new("Fit to Grid (G)"))
        .clicked()
    {
        for (_entity, _item, mut transform, sprite, _render_layers) in selected_query.iter_mut() {
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
    if ui
        .add_sized([140.0, 26.0], egui::Button::new("Center to Grid (Shift+G)"))
        .clicked()
    {
        let grid_size = map_data.grid_size;
        let half = grid_size / 2.0;
        for (_entity, _item, mut transform, _sprite, _render_layers) in selected_query.iter_mut() {
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
        for (_entity, _item, mut transform, _sprite, _render_layers) in selected_query.iter_mut() {
            let uniform_scale = transform.scale.x.abs().max(transform.scale.y.abs());
            transform.scale.x = uniform_scale;
            transform.scale.y = uniform_scale;
        }
    }
}

/// Renders UI for a single selected item with full property controls.
fn render_single_selection(
    ui: &mut egui::Ui,
    map_data: &MapData,
    selected_query: &mut SelectedQuery,
    images: &Assets<Image>,
) {
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
                            transform.translation.z = layer.z_base() + item.z_index as f32;
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
                    transform.translation.z = item.layer.z_base() + item.z_index as f32;
                }
            });

            ui.add_enabled_ui(item.z_index > 0, |ui| {
                if ui.small_button("-").clicked() {
                    item.z_index -= 1;
                    transform.translation.z = item.layer.z_base() + item.z_index as f32;
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
                .add(
                    egui::DragValue::new(&mut rotation_deg)
                        .speed(1.0)
                        .suffix("°"),
                )
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

        if ui
            .add_sized([140.0, 26.0], egui::Button::new("Fit to Grid (G)"))
            .clicked()
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

        ui.add_space(4.0);
        if ui
            .add_sized([140.0, 26.0], egui::Button::new("Center to Grid (Shift+G)"))
            .clicked()
        {
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
        if ui
            .add_sized([140.0, 26.0], egui::Button::new("Restore Aspect Ratio (A)"))
            .clicked()
        {
            let uniform_scale = transform.scale.x.abs().max(transform.scale.y.abs());
            transform.scale.x = uniform_scale;
            transform.scale.y = uniform_scale;
        }
    }
}
