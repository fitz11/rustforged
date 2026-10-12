//! Properties panel UI for selected items.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::assets::RetargetAssetRequest;
use crate::editor::SelectionEdit;
use crate::editor::history::MapObject;
use crate::map::{ItemId, Layer, PlacedItem, Selected};

/// Selected item query type for the properties panel.
pub type SelectedQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ItemId,
        &'static mut PlacedItem,
        &'static mut Transform,
        &'static mut RenderLayers,
    ),
    With<Selected>,
>;

/// What the user did in the Properties panel this frame.
#[derive(Default)]
pub struct PropertiesOutput {
    /// Relink the selected item's missing asset
    pub relink: Option<RetargetAssetRequest>,
    /// A button for an edit that also has a keyboard shortcut
    pub selection_edit: Option<SelectionEdit>,
    /// The single selected item before and after this frame's field edits
    pub item_edit: Option<ItemEdit>,
}

/// The single selected item before and after this frame's edits in the panel.
pub struct ItemEdit {
    pub before: MapObject,
    pub after: MapObject,
    /// A field is still being dragged or typed in, so the edit isn't finished
    pub editing: bool,
}

/// Renders the properties section for selected items.
///
/// `missing` lists selected items whose asset is missing; `library_selection` is the asset
/// selected in the library browser.
pub fn render_properties(
    ui: &mut egui::Ui,
    selected_query: &mut SelectedQuery,
    missing: &[Entity],
    library_selection: Option<&str>,
) -> PropertiesOutput {
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Properties").heading().size(18.0));
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    let selected_count = selected_query.iter().count();
    let mut output = PropertiesOutput::default();

    if selected_count == 0 {
        ui.label(egui::RichText::new("No item selected").size(14.0).weak());
    } else if selected_count > 1 {
        ui.label(egui::RichText::new(format!("{} items selected", selected_count)).size(14.0));
        ui.add_space(8.0);
        output.selection_edit = render_selection_edit_buttons(ui);
    } else {
        render_single_selection(ui, selected_query, missing, library_selection, &mut output);
    }
    output
}

/// Explains a missing asset and offers to relink it to the asset selected in the library.
fn render_missing_asset(
    ui: &mut egui::Ui,
    asset_path: &str,
    library_selection: Option<&str>,
) -> Option<RetargetAssetRequest> {
    ui.colored_label(
        egui::Color32::from_rgb(220, 90, 90),
        egui::RichText::new("Asset missing from library").size(13.0),
    );
    ui.label(egui::RichText::new(asset_path).small().weak());
    ui.add_space(4.0);

    let button = ui
        .add_enabled(
            library_selection.is_some(),
            egui::Button::new("Relink to selected asset"),
        )
        .on_hover_text("Use the asset selected in the library for every item that uses this one")
        .on_disabled_hover_text("Select an asset in the library first");
    ui.add_space(8.0);

    match library_selection {
        Some(new_path) if button.clicked() => Some(RetargetAssetRequest {
            old_path: asset_path.to_string(),
            new_path: new_path.to_string(),
        }),
        _ => None,
    }
}

/// Fit, Center and Restore Aspect Ratio, which work the same as their shortcuts
fn render_selection_edit_buttons(ui: &mut egui::Ui) -> Option<SelectionEdit> {
    let mut edit = None;
    if ui
        .add_sized([140.0, 26.0], egui::Button::new("Fit to Grid (G)"))
        .clicked()
    {
        edit = Some(SelectionEdit::FitToGrid);
    }
    ui.add_space(4.0);
    if ui
        .add_sized([140.0, 26.0], egui::Button::new("Center to Grid (Shift+G)"))
        .clicked()
    {
        edit = Some(SelectionEdit::CenterToGrid);
    }
    ui.add_space(4.0);
    if ui
        .add_sized([140.0, 26.0], egui::Button::new("Restore Aspect Ratio (A)"))
        .clicked()
    {
        edit = Some(SelectionEdit::RestoreAspectRatio);
    }
    edit
}

/// A labelled number field. Returns whether it's still being dragged or typed in.
fn number_field(ui: &mut egui::Ui, label: &str, value: &mut f32, speed: f64, suffix: &str) -> bool {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(14.0));
        let response = ui.add(egui::DragValue::new(value).speed(speed).suffix(suffix));
        response.dragged() || response.has_focus()
    })
    .inner
}

/// Renders UI for a single selected item with full property controls.
fn render_single_selection(
    ui: &mut egui::Ui,
    selected_query: &mut SelectedQuery,
    missing: &[Entity],
    library_selection: Option<&str>,
    output: &mut PropertiesOutput,
) {
    let Ok((entity, id, mut item, mut transform, mut render_layers)) = selected_query.single_mut()
    else {
        return;
    };

    // Asset path (truncated if too long)
    let asset_name = item
        .asset_path
        .split('/')
        .next_back()
        .unwrap_or(&item.asset_path);
    ui.label(egui::RichText::new(format!("Asset: {}", asset_name)).size(13.0));

    ui.add_space(8.0);

    if missing.contains(&entity) {
        output.relink = render_missing_asset(ui, &item.asset_path, library_selection);
    }

    // Edit copies, so an untouched panel doesn't mark the item changed every frame
    let before = MapObject::item(*id, item.clone(), *transform);
    let mut new_item = item.clone();
    let mut new_transform = *transform;
    let mut editing = false;

    // Layer selector
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Layer:").size(14.0));
        egui::ComboBox::from_id_salt("item_layer")
            .selected_text(new_item.layer.display_name())
            .show_ui(ui, |ui| {
                for layer in Layer::all() {
                    // Skip FogOfWar layer (not for placing items)
                    if *layer == Layer::FogOfWar {
                        continue;
                    }
                    let is_selected = new_item.layer == *layer;
                    if ui
                        .selectable_label(is_selected, layer.display_name())
                        .clicked()
                    {
                        new_item.layer = *layer;
                    }
                }
            });
    });

    ui.add_space(4.0);

    // Z-Index property
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Z-Index:").size(14.0));
        ui.label(egui::RichText::new(format!("{}", new_item.z_index)).size(14.0));

        let max_z = Layer::max_z_index();

        ui.add_enabled_ui(new_item.z_index < max_z, |ui| {
            if ui.small_button("+").clicked() {
                new_item.z_index += 1;
            }
        });

        ui.add_enabled_ui(new_item.z_index > 0, |ui| {
            if ui.small_button("-").clicked() {
                new_item.z_index -= 1;
            }
        });
    });

    ui.add_space(8.0);

    // Position
    editing |= number_field(ui, "X:", &mut new_transform.translation.x, 1.0, "");
    ui.add_space(2.0);
    editing |= number_field(ui, "Y:", &mut new_transform.translation.y, 1.0, "");

    ui.add_space(8.0);

    // Rotation
    // EulerRot::ZYX returns (z, y, x) - we want the Z rotation (first component)
    let (rotation, _, _) = new_transform.rotation.to_euler(EulerRot::ZYX);
    let mut rotation_deg = rotation.to_degrees();
    let rotation_before = rotation_deg;
    editing |= number_field(ui, "Rotation:", &mut rotation_deg, 1.0, "°");
    if rotation_deg != rotation_before {
        new_transform.rotation = Quat::from_rotation_z(rotation_deg.to_radians());
    }

    ui.add_space(8.0);

    // Scale
    editing |= number_field(ui, "Scale X:", &mut new_transform.scale.x, 0.01, "");
    ui.add_space(2.0);
    editing |= number_field(ui, "Scale Y:", &mut new_transform.scale.y, 0.01, "");

    ui.add_space(8.0);
    output.selection_edit = render_selection_edit_buttons(ui);

    let after = MapObject::item(*id, new_item.clone(), new_transform);
    if after != before {
        if new_item != *item {
            *render_layers = new_item.layer.render_layers();
            *item = new_item;
        }
        if let MapObject::Item {
            transform: normalized,
            ..
        } = &after
        {
            // At the item's depth for its (possibly new) layer and z-index
            *transform = *normalized;
        }
    }
    output.item_edit = Some(ItemEdit {
        before,
        after,
        editing,
    });
}
