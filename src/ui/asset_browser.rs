use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::assets::{AssetCategory, AssetLibrary, LibraryAsset, SelectedAsset};
use crate::editor::{CurrentTool, EditorTool};

#[derive(Resource, Default)]
pub struct AssetBrowserState {
    pub selected_category: AssetCategory,
}

pub fn asset_browser_ui(
    mut contexts: EguiContexts,
    library: Res<AssetLibrary>,
    mut selected_asset: ResMut<SelectedAsset>,
    mut browser_state: ResMut<AssetBrowserState>,
    mut current_tool: ResMut<CurrentTool>,
) -> Result {
    egui::SidePanel::left("asset_browser")
        .default_width(200.0)
        .show(contexts.ctx_mut()?, |ui| {
            ui.heading("Asset Library");

            ui.horizontal(|ui| {
                for category in AssetCategory::all() {
                    let selected = browser_state.selected_category == *category;
                    if ui
                        .selectable_label(selected, category.display_name())
                        .clicked()
                    {
                        browser_state.selected_category = *category;
                    }
                }
            });

            ui.separator();

            let filtered_assets: Vec<&LibraryAsset> = library
                .assets
                .iter()
                .filter(|a| a.category == browser_state.selected_category)
                .collect();

            if filtered_assets.is_empty() {
                ui.label("No assets found.");
                ui.label(format!(
                    "Add images to assets/library/{}/",
                    browser_state.selected_category.folder_name()
                ));
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for asset in filtered_assets {
                        let is_selected = selected_asset
                            .asset
                            .as_ref()
                            .map(|a| a.relative_path == asset.relative_path)
                            .unwrap_or(false);

                        if ui.selectable_label(is_selected, &asset.name).clicked() {
                            selected_asset.asset = Some(asset.clone());
                            current_tool.tool = EditorTool::Place;
                        }
                    }
                });
            }

            ui.separator();

            if let Some(ref asset) = selected_asset.asset {
                ui.label(format!("Selected: {}", asset.name));
            } else {
                ui.label("No asset selected");
            }
        });
    Ok(())
}
