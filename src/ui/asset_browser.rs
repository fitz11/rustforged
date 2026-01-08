use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::path::PathBuf;

use crate::assets::{
    create_and_open_library, open_library_directory, AssetCategory, AssetLibrary, LibraryAsset,
    SelectedAsset,
};
use crate::editor::{CurrentTool, EditorTool};
use crate::map::{LoadMapRequest, MapData};

use super::asset_import::AssetImportDialog;
use super::file_menu::FileMenuState;

#[derive(Resource, Default)]
pub struct AssetBrowserState {
    pub selected_category: AssetCategory,
    /// Whether the library info section is expanded
    pub library_expanded: bool,
}

pub fn asset_browser_ui(
    mut contexts: EguiContexts,
    mut library: ResMut<AssetLibrary>,
    mut selected_asset: ResMut<SelectedAsset>,
    mut browser_state: ResMut<AssetBrowserState>,
    mut current_tool: ResMut<CurrentTool>,
    mut menu_state: ResMut<FileMenuState>,
    mut load_events: MessageWriter<LoadMapRequest>,
    mut import_dialog: ResMut<AssetImportDialog>,
    map_data: Res<MapData>,
) -> Result {
    egui::SidePanel::left("asset_browser")
        .default_width(200.0)
        .show(contexts.ctx_mut()?, |ui| {
            // =========================================
            // FILE/ASSETS MENU SECTION
            // =========================================
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Map").clicked() {
                        menu_state.show_new_confirmation = true;
                        ui.close();
                    }

                    ui.separator();

                    if ui.button("Save Map...").clicked() {
                        menu_state.save_filename = map_data.name.clone();
                        menu_state.show_save_name_dialog = true;
                        ui.close();
                    }

                    if ui.button("Load Map...").clicked() {
                        let maps_dir = PathBuf::from("assets/maps");
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Map Files", &["json"])
                            .set_directory(&maps_dir)
                            .set_title("Load Map")
                            .pick_file()
                        {
                            load_events.write(LoadMapRequest { path });
                        }
                        ui.close();
                    }
                });

                ui.menu_button("Assets", |ui| {
                    if ui.button("Import Assets...").clicked() {
                        import_dialog.is_open = true;
                        ui.close();
                    }
                });
            });

            ui.separator();

            // =========================================
            // ASSET LIBRARY DIRECTORY SECTION
            // =========================================
            ui.horizontal(|ui| {
                let toggle_text = if browser_state.library_expanded {
                    "▼"
                } else {
                    "▶"
                };
                if ui.small_button(toggle_text).clicked() {
                    browser_state.library_expanded = !browser_state.library_expanded;
                }
                ui.heading("Asset Library");
            });

            // Show current library path (truncated if too long)
            let path_str = library.library_path.to_string_lossy();
            let display_path = if path_str.len() > 30 {
                format!("...{}", &path_str[path_str.len() - 27..])
            } else {
                path_str.to_string()
            };
            ui.label(egui::RichText::new(display_path).small().weak())
                .on_hover_text(path_str.as_ref());

            // Show error if any
            if let Some(ref error) = library.error {
                ui.colored_label(egui::Color32::RED, egui::RichText::new(error).small());
            }

            // Library management buttons (shown when expanded)
            if browser_state.library_expanded {
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("Open...").clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .set_title("Open Asset Library")
                            .pick_folder()
                        && let Err(e) = open_library_directory(&mut library, path)
                    {
                        warn!("Failed to open library: {}", e);
                    }

                    if ui.button("New...").clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .set_title("Create New Asset Library")
                            .pick_folder()
                        && let Err(e) = create_and_open_library(&mut library, path)
                    {
                        warn!("Failed to create library: {}", e);
                    }
                });

                ui.add_space(4.0);
            }

            ui.separator();

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
                let folder_path = library
                    .library_path
                    .join(browser_state.selected_category.folder_name());
                ui.label(format!("Add images to {}/", folder_path.display()));
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
