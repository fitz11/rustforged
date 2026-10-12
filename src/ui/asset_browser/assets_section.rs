//! Assets section of the asset browser: folders, the asset list and the selected asset.

use bevy::prelude::*;
use bevy_egui::egui;

use crate::assets::{
    AssetLibrary, LibraryAsset, SelectedAsset, THUMBNAIL_SIZE, ThumbnailCache, get_image_dimensions,
};
use crate::editor::{CurrentTool, EditorTool};

use super::helpers::{extension_color, indented};
use super::state::{AssetBrowserState, DialogStates};

/// Render the assets management buttons.
pub(super) fn render_assets_buttons(
    ui: &mut egui::Ui,
    library: &AssetLibrary,
    dialogs: &mut DialogStates,
) {
    ui.label(egui::RichText::new("Assets").size(13.0).strong());
    ui.separator();

    ui.horizontal(|ui| {
        if ui
            .add_sized([80.0, 24.0], egui::Button::new("Import"))
            .clicked()
        {
            dialogs.import_dialog.is_open = true;
        }
        if ui
            .add_sized([80.0, 24.0], egui::Button::new("Open Folder"))
            .on_hover_text("Open library folder in file explorer")
            .clicked()
        {
            let path = &library.library_path;
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open").arg(path).spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(path).spawn();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer").arg(path).spawn();
            }
        }
    });
}

/// Render the folder tree view. Each folder level is indented one step. With no folder
/// selected, the asset list shows the library's top-level assets; clicking the selected
/// folder again goes back to them.
pub(super) fn render_folder_tree(ui: &mut egui::Ui, browser_state: &mut AssetBrowserState) {
    ui.label(egui::RichText::new("Folders").size(12.0).weak());
    egui::ScrollArea::vertical()
        .id_salt("folder_tree")
        .max_height(120.0)
        .show(ui, |ui| {
            for folder in &browser_state.discovered_folders.clone() {
                let is_selected = browser_state.selected_folder == *folder;
                let depth = folder.matches('/').count();
                let display_name = folder.split('/').next_back().unwrap_or(folder);

                let response = ui.selectable_label(
                    is_selected,
                    egui::RichText::new(indented(display_name, depth)).size(12.0),
                );
                let response = if is_selected {
                    response.on_hover_text("Click again to show the library's top-level assets")
                } else {
                    response
                };
                if response.clicked() {
                    browser_state.selected_folder = if is_selected {
                        String::new()
                    } else {
                        folder.clone()
                    };
                }
            }
        });
}

/// Render the asset list for the selected folder.
pub(super) fn render_asset_list(
    ui: &mut egui::Ui,
    library: &AssetLibrary,
    selected_asset: &mut SelectedAsset,
    current_tool: &mut CurrentTool,
    browser_state: &mut AssetBrowserState,
    thumbnail_cache: &ThumbnailCache,
) {
    let filtered_assets: Vec<&LibraryAsset> = library
        .assets
        .iter()
        .filter(|a| a.folder_path == browser_state.selected_folder)
        .collect();

    if filtered_assets.is_empty() {
        ui.label("No assets in this folder.");
        let folder_display = if browser_state.selected_folder.is_empty() {
            library.library_path.display().to_string()
        } else {
            library
                .library_path
                .join(&browser_state.selected_folder)
                .display()
                .to_string()
        };
        ui.label(format!("Add images to {}/", folder_display));
    } else {
        // Only the rows in view are laid out, and only they get thumbnails
        let row_height =
            (THUMBNAIL_SIZE as f32).max(ui.spacing().interact_size.y) + ui.spacing().item_spacing.y;
        browser_state.visible_assets.clear();
        egui::ScrollArea::vertical().show_rows(
            ui,
            row_height,
            filtered_assets.len(),
            |ui, rows| {
                for asset in &filtered_assets[rows] {
                    browser_state.visible_assets.push(asset.full_path.clone());
                    render_asset_row(
                        ui,
                        asset,
                        selected_asset,
                        current_tool,
                        browser_state,
                        thumbnail_cache,
                    );
                }
            },
        );
    }
}

/// Render a single asset row with thumbnail.
pub(super) fn render_asset_row(
    ui: &mut egui::Ui,
    asset: &LibraryAsset,
    selected_asset: &mut SelectedAsset,
    current_tool: &mut CurrentTool,
    browser_state: &mut AssetBrowserState,
    thumbnail_cache: &ThumbnailCache,
) {
    let is_selected = selected_asset
        .asset
        .as_ref()
        .map(|a| a.path == asset.path)
        .unwrap_or(false);

    ui.horizontal(|ui| {
        let thumb_size = THUMBNAIL_SIZE as f32;

        if let Some(texture_id) = thumbnail_cache.get_texture_id(&asset.full_path) {
            ui.add(
                egui::Image::new(egui::load::SizedTexture::new(
                    texture_id,
                    egui::vec2(thumb_size, thumb_size),
                ))
                .fit_to_exact_size(egui::vec2(thumb_size, thumb_size))
                .corner_radius(2.0),
            );
        } else {
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(thumb_size, thumb_size), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, 2.0, egui::Color32::from_rgb(60, 60, 60));
        }

        // The library scan and watcher keep the list current; a file that can't be read
        // shows up as a failed thumbnail
        let is_missing = thumbnail_cache.has_failed(&asset.full_path);

        let label_text = if is_missing {
            egui::RichText::new(&asset.name).color(egui::Color32::from_rgb(200, 100, 100))
        } else {
            egui::RichText::new(&asset.name)
        };

        let response = ui.selectable_label(is_selected, label_text);
        let response = if is_missing {
            response.on_hover_text("Asset file can't be read")
        } else {
            response
        };

        if response.clicked() {
            if !asset.full_path.exists() {
                warn!("Selected asset no longer exists: {:?}", asset.full_path);
            } else {
                browser_state.selected_dimensions = None;
                browser_state.cached_dimensions_path = None;
                selected_asset.asset = Some(asset.clone());
                current_tool.tool = EditorTool::Place;
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ext_short = asset
                .extension
                .chars()
                .take(3)
                .collect::<String>()
                .to_uppercase();
            ui.label(
                egui::RichText::new(ext_short)
                    .small()
                    .color(extension_color(&asset.extension)),
            );
        });
    });
}

/// Render the selected asset information panel.
pub(super) fn render_selected_asset_info(
    ui: &mut egui::Ui,
    selected_asset: &SelectedAsset,
    browser_state: &mut AssetBrowserState,
) {
    if let Some(ref asset) = selected_asset.asset {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Selected Asset").size(14.0).strong());
            if ui
                .small_button("Rename")
                .on_hover_text("Rename asset (F2)")
                .clicked()
            {
                browser_state.rename_new_name = asset.name.clone();
                browser_state.rename_error = None;
                browser_state.rename_dialog_open = true;
            }
            if ui
                .small_button("Move")
                .on_hover_text("Move to another category")
                .clicked()
            {
                browser_state.move_error = None;
                browser_state.move_dialog_open = true;
            }
        });
        ui.add_space(6.0);

        ui.label(egui::RichText::new(&asset.name).size(13.0));
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Type:").size(13.0).weak());
            ui.label(
                egui::RichText::new(asset.extension.to_uppercase())
                    .size(13.0)
                    .strong(),
            );
        });

        let needs_dimension_load = browser_state
            .cached_dimensions_path
            .as_ref()
            .map(|p| p != &asset.full_path)
            .unwrap_or(true);

        if needs_dimension_load {
            browser_state.selected_dimensions = get_image_dimensions(&asset.full_path);
            browser_state.cached_dimensions_path = Some(asset.full_path.clone());
        }

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Size:").size(13.0).weak());
            if let Some((width, height)) = browser_state.selected_dimensions {
                ui.label(
                    egui::RichText::new(format!("{}x{}", width, height))
                        .size(13.0)
                        .strong(),
                );
            } else {
                ui.label(egui::RichText::new("Unknown").size(13.0).weak());
            }
        });
    } else {
        ui.label(egui::RichText::new("No asset selected").size(13.0).weak());
    }
}
