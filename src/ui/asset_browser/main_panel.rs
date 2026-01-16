//! Main asset browser panel UI.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::assets::{
    create_and_open_library, get_image_dimensions, open_library_directory, AssetLibrary,
    LibraryAsset, RefreshAssetLibrary, RenameAssetRequest, SelectedAsset, ThumbnailCache,
    UpdateLibraryMetadataRequest, THUMBNAIL_SIZE,
};
use crate::config::{AppConfig, SetDefaultLibraryRequest};
use crate::editor::{CurrentTool, EditorTool};
use crate::map::SwitchMapRequest;

use super::dialogs::{
    handle_rename_shortcuts, render_import_error_dialog, render_move_asset_dialog,
    render_rename_asset_dialog, render_rename_library_dialog, render_rename_map_dialog,
    render_set_default_dialog, render_success_dialog,
};
use super::helpers::{discover_folders, extension_color, sanitize_map_name, scan_maps_directory};
use super::library_ops::{export_library_to_zip, import_library_from_zip};
use super::state::{AssetBrowserState, DialogStates, MapResources};

/// Main asset browser UI system.
#[allow(clippy::too_many_arguments)]
pub fn asset_browser_ui(
    mut contexts: EguiContexts,
    mut library: ResMut<AssetLibrary>,
    mut selected_asset: ResMut<SelectedAsset>,
    mut browser_state: ResMut<AssetBrowserState>,
    mut current_tool: ResMut<CurrentTool>,
    mut thumbnail_cache: ResMut<ThumbnailCache>,
    config: Res<AppConfig>,
    mut set_default_events: MessageWriter<SetDefaultLibraryRequest>,
    mut rename_events: MessageWriter<RenameAssetRequest>,
    mut library_metadata_events: MessageWriter<UpdateLibraryMetadataRequest>,
    mut refresh_events: MessageWriter<RefreshAssetLibrary>,
    mut map_res: MapResources,
    mut dialogs: DialogStates,
) -> Result {
    // Clear thumbnail cache and update folders if library path changed
    let current_path = library.library_path.clone();
    if browser_state.last_library_path.as_ref() != Some(&current_path) {
        thumbnail_cache.clear();
        browser_state.discovered_folders = discover_folders(&library);
        browser_state.last_library_path = Some(current_path);
    }

    // Handle keyboard shortcuts for rename dialogs
    handle_rename_shortcuts(
        &mut contexts,
        &mut browser_state,
        &selected_asset,
        &map_res,
        &library,
    );

    egui::SidePanel::left("asset_browser")
        .default_width(220.0)
        .show(contexts.ctx_mut()?, |ui| {
            render_library_section(
                ui,
                &mut library,
                &mut browser_state,
                &mut dialogs,
                &mut map_res,
                &mut thumbnail_cache,
            );

            ui.separator();
            ui.add_space(4.0);

            render_folder_tree(ui, &mut browser_state);

            ui.add_space(4.0);
            ui.separator();

            render_asset_list(
                ui,
                &library,
                &mut selected_asset,
                &mut current_tool,
                &mut browser_state,
                &thumbnail_cache,
            );

            ui.separator();
            ui.add_space(4.0);

            render_selected_asset_info(ui, &selected_asset, &mut browser_state);

            // Settings button at bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                if ui
                    .add_sized([100.0, 24.0], egui::Button::new("Settings"))
                    .clicked()
                {
                    dialogs.settings_state.load_from_config(&config);
                    dialogs.settings_state.load_library_info(&library);
                    dialogs.settings_state.is_open = true;
                }
                ui.add_space(4.0);
                ui.separator();
            });
        });

    // Render all dialogs
    let ctx = contexts.ctx_mut()?;
    render_set_default_dialog(ctx, &mut browser_state, &mut set_default_events);
    render_import_error_dialog(ctx, &mut browser_state);
    render_success_dialog(ctx, &mut browser_state);
    render_rename_asset_dialog(
        ctx,
        &mut browser_state,
        &mut selected_asset,
        &mut library,
        &mut thumbnail_cache,
        &mut rename_events,
    );
    render_rename_map_dialog(ctx, &mut browser_state, &mut map_res);
    render_rename_library_dialog(ctx, &mut browser_state, &mut library_metadata_events);
    render_move_asset_dialog(
        ctx,
        &mut browser_state,
        &mut selected_asset,
        &mut library,
        &mut thumbnail_cache,
        &mut rename_events,
    );

    // Handle library refresh request
    if browser_state.refresh_requested {
        browser_state.refresh_requested = false;
        thumbnail_cache.clear();
        browser_state.discovered_folders = discover_folders(&library);
        refresh_events.write(RefreshAssetLibrary);
    }

    Ok(())
}

/// Render the library management section.
fn render_library_section(
    ui: &mut egui::Ui,
    library: &mut AssetLibrary,
    browser_state: &mut AssetBrowserState,
    dialogs: &mut DialogStates,
    map_res: &mut MapResources,
    _thumbnail_cache: &mut ThumbnailCache,
) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let toggle_text = if browser_state.library_expanded {
            "v"
        } else {
            ">"
        };
        if ui.button(toggle_text).clicked() {
            browser_state.library_expanded = !browser_state.library_expanded;
        }
        ui.label(egui::RichText::new(&library.metadata.name).heading().size(18.0));
    });
    ui.add_space(2.0);

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

    // Library management and subsections (shown when expanded)
    if browser_state.library_expanded {
        ui.add_space(6.0);
        render_library_buttons(ui, library, browser_state);
        render_export_import_buttons(ui, library, browser_state);
        ui.add_space(10.0);
        render_maps_section(ui, library, browser_state, dialogs, map_res);
        ui.add_space(10.0);
        render_assets_buttons(ui, library, dialogs);
        ui.add_space(6.0);
    }
}

/// Render library management buttons (Open, Refresh, New, Rename).
fn render_library_buttons(
    ui: &mut egui::Ui,
    library: &mut AssetLibrary,
    browser_state: &mut AssetBrowserState,
) {
    ui.horizontal(|ui| {
        if ui
            .add_sized([50.0, 24.0], egui::Button::new("Open"))
            .on_hover_text("Open existing library folder")
            .clicked()
            && let Some(path) = rfd::FileDialog::new()
                .set_title("Open Asset Library")
                .pick_folder()
        {
            if let Err(e) = open_library_directory(library, path.clone()) {
                warn!("Failed to open library: {}", e);
            } else {
                browser_state.show_set_default_dialog = true;
                browser_state.set_default_dialog_path = Some(path);
                browser_state.set_as_default_checked = false;
            }
        }

        if ui
            .add_sized([24.0, 24.0], egui::Button::new("\u{21bb}"))
            .on_hover_text("Refresh library")
            .clicked()
        {
            browser_state.refresh_requested = true;
        }

        if ui
            .add_sized([50.0, 24.0], egui::Button::new("New"))
            .on_hover_text("Create new library folder")
            .clicked()
            && let Some(path) = rfd::FileDialog::new()
                .set_title("Create New Asset Library")
                .pick_folder()
        {
            if let Err(e) = create_and_open_library(library, path.clone()) {
                warn!("Failed to create library: {}", e);
            } else {
                browser_state.show_set_default_dialog = true;
                browser_state.set_default_dialog_path = Some(path);
                browser_state.set_as_default_checked = false;
            }
        }

        if ui
            .add_sized([65.0, 24.0], egui::Button::new("Rename"))
            .on_hover_text("Rename library (F4)")
            .clicked()
        {
            browser_state.rename_library_new_name = library.metadata.name.clone();
            browser_state.rename_library_dialog_open = true;
        }
    });
}

/// Render export/import library buttons.
fn render_export_import_buttons(
    ui: &mut egui::Ui,
    library: &mut AssetLibrary,
    browser_state: &mut AssetBrowserState,
) {
    ui.horizontal(|ui| {
        if ui.add_sized([70.0, 24.0], egui::Button::new("Export...")).clicked()
            && let Some(dest_path) = rfd::FileDialog::new()
                .set_title("Export Library as Zip")
                .set_file_name(format!("{}.zip", library.metadata.name))
                .add_filter("Zip Archive", &["zip"])
                .save_file()
        {
            match export_library_to_zip(&library.library_path, &dest_path) {
                Ok(()) => {
                    browser_state.library_operation_success =
                        Some(format!("Library exported to:\n{}", dest_path.display()));
                }
                Err(e) => {
                    browser_state.library_import_error = Some(e);
                }
            }
        }

        if ui.add_sized([70.0, 24.0], egui::Button::new("Import...")).clicked()
            && let Some(zip_path) = rfd::FileDialog::new()
                .set_title("Import Library from Zip")
                .add_filter("Zip Archive", &["zip"])
                .pick_file()
            && let Some(dest_path) = rfd::FileDialog::new()
                .set_title("Select Destination Folder")
                .pick_folder()
        {
            match import_library_from_zip(&zip_path, &dest_path) {
                Ok(()) => {
                    if let Err(e) = open_library_directory(library, dest_path.clone()) {
                        browser_state.library_import_error =
                            Some(format!("Library extracted but failed to open: {}", e));
                    } else {
                        browser_state.library_operation_success =
                            Some("Library imported successfully!".to_string());
                        browser_state.show_set_default_dialog = true;
                        browser_state.set_default_dialog_path = Some(dest_path);
                        browser_state.set_as_default_checked = false;
                    }
                }
                Err(e) => {
                    browser_state.library_import_error = Some(e);
                }
            }
        }
    });
}

/// Render the maps subsection.
fn render_maps_section(
    ui: &mut egui::Ui,
    library: &AssetLibrary,
    browser_state: &mut AssetBrowserState,
    dialogs: &mut DialogStates,
    map_res: &mut MapResources,
) {
    ui.label(egui::RichText::new("Maps").size(13.0).strong());
    ui.separator();

    // Show open maps with unsaved indicators
    let mut map_to_switch: Option<u64> = None;

    if !map_res.open_maps.maps.is_empty() {
        ui.label(egui::RichText::new("Open:").size(12.0).weak());

        let mut sorted_maps: Vec<_> = map_res.open_maps.maps.values().collect();
        sorted_maps.sort_by_key(|m| m.id);

        for map in sorted_maps {
            let is_active = map_res.open_maps.active_map_id == Some(map.id);
            let display_name = if map.is_dirty {
                format!("{}*", map.name)
            } else {
                map.name.clone()
            };

            ui.horizontal(|ui| {
                if is_active {
                    ui.label(egui::RichText::new(">").size(10.0));
                } else {
                    ui.add_space(12.0);
                }

                let text = if is_active {
                    egui::RichText::new(&display_name).size(12.0).strong()
                } else {
                    egui::RichText::new(&display_name).size(12.0)
                };

                if is_active {
                    ui.label(text);
                } else if ui
                    .add(egui::Button::new(text).frame(false))
                    .on_hover_text("Click to switch to this map")
                    .clicked()
                {
                    map_to_switch = Some(map.id);
                }

                if map.path.is_some() && map.is_dirty {
                    ui.label(
                        egui::RichText::new("(modified)")
                            .size(10.0)
                            .weak()
                            .italics(),
                    );
                } else if map.path.is_none() {
                    ui.label(egui::RichText::new("(new)").size(10.0).weak().italics());
                }
            });
        }
        ui.add_space(4.0);
    } else {
        // Fallback to showing current map indicator
        if let Some(ref current_path) = map_res.current_map_file.path {
            let current_name = current_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");
            let dirty_indicator = if map_res.dirty_state.is_dirty {
                "*"
            } else {
                ""
            };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Current:").size(12.0).weak());
                ui.label(
                    egui::RichText::new(format!("{}{}", current_name, dirty_indicator))
                        .size(12.0)
                        .strong(),
                );
            });
            ui.add_space(4.0);
        } else {
            let dirty_indicator = if map_res.dirty_state.is_dirty {
                "*"
            } else {
                ""
            };
            ui.label(
                egui::RichText::new(format!("(unsaved map){}", dirty_indicator))
                    .size(12.0)
                    .weak()
                    .italics(),
            );
            ui.add_space(4.0);
        }
    }

    if let Some(target_id) = map_to_switch {
        map_res
            .switch_events
            .write(SwitchMapRequest { map_id: target_id });
    }

    ui.horizontal(|ui| {
        if ui
            .add_sized([45.0, 24.0], egui::Button::new("New"))
            .clicked()
        {
            dialogs.menu_state.show_new_confirmation = true;
        }
        if ui
            .add_sized([45.0, 24.0], egui::Button::new("Save"))
            .clicked()
        {
            // Only prompt for name if map is untitled; otherwise save directly
            let active_map = map_res.open_maps.maps.get(&map_res.open_maps.active_map_id.unwrap_or(0));
            if let Some(active) = active_map {
                if active.name == "Untitled Map" {
                    dialogs.menu_state.save_filename = active.name.clone();
                    dialogs.menu_state.show_save_name_dialog = true;
                } else if let Some(ref existing_path) = active.path {
                    // Map has been saved before, save to same path
                    map_res
                        .save_events
                        .write(crate::map::SaveMapRequest { path: existing_path.clone() });
                } else {
                    // Map has a name but no path yet - save to maps directory
                    let maps_dir = library.library_path.join("maps");
                    let filename = sanitize_map_name(&active.name);
                    let path = maps_dir.join(format!("{}.json", filename));
                    map_res
                        .save_events
                        .write(crate::map::SaveMapRequest { path });
                }
            }
        }
        if ui
            .add_sized([55.0, 24.0], egui::Button::new("Rename"))
            .on_hover_text("Rename map (F3)")
            .clicked()
        {
            browser_state.rename_map_new_name = map_res.map_data.name.clone();
            browser_state.rename_map_dialog_open = true;
        }
    });

    // Scan and show available maps
    let maps_dir = library.library_path.join("maps");
    if !maps_dir.exists()
        && let Err(e) = std::fs::create_dir_all(&maps_dir)
    {
        warn!("Failed to create maps directory: {}", e);
    }

    if browser_state.last_maps_scan_path.as_ref() != Some(&maps_dir) {
        browser_state.cached_maps = scan_maps_directory(&maps_dir);
        browser_state.last_maps_scan_path = Some(maps_dir.clone());
    }

    if !browser_state.cached_maps.is_empty() {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Available:").size(12.0).weak());
            if ui
                .small_button("R")
                .on_hover_text("Refresh map list")
                .clicked()
            {
                browser_state.cached_maps = scan_maps_directory(&maps_dir);
            }
        });

        egui::ScrollArea::vertical()
            .id_salt("maps_scroll")
            .max_height(120.0)
            .show(ui, |ui| {
                for (map_name, map_path) in &browser_state.cached_maps {
                    let is_current = map_res
                        .current_map_file
                        .path
                        .as_ref()
                        .map(|p| p == map_path)
                        .unwrap_or(false);

                    ui.horizontal(|ui| {
                        if is_current {
                            ui.label(egui::RichText::new(">").size(10.0));
                        } else {
                            ui.add_space(12.0);
                        }

                        let button_text = if is_current {
                            egui::RichText::new(map_name).size(12.0).strong()
                        } else {
                            egui::RichText::new(map_name).size(12.0)
                        };

                        if ui
                            .add(egui::Button::new(button_text).frame(false))
                            .on_hover_text(map_path.to_string_lossy().as_ref())
                            .clicked()
                            && !is_current
                        {
                            map_res.load_events.write(crate::map::LoadMapRequest {
                                path: map_path.clone(),
                            });
                        }
                    });
                }
            });
    }
}

/// Render the assets management buttons.
fn render_assets_buttons(
    ui: &mut egui::Ui,
    library: &AssetLibrary,
    dialogs: &mut DialogStates,
) {
    ui.label(egui::RichText::new("Assets").size(13.0).strong());
    ui.separator();

    ui.horizontal(|ui| {
        if ui
            .add_sized([80.0, 24.0], egui::Button::new("Import..."))
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

/// Render the folder tree view.
fn render_folder_tree(ui: &mut egui::Ui, browser_state: &mut AssetBrowserState) {
    ui.label(egui::RichText::new("Folders").size(12.0).weak());
    egui::ScrollArea::vertical()
        .id_salt("folder_tree")
        .max_height(120.0)
        .show(ui, |ui| {
            let root_selected = browser_state.selected_folder.is_empty();
            if ui
                .selectable_label(root_selected, egui::RichText::new("(root)").size(12.0))
                .clicked()
            {
                browser_state.selected_folder = String::new();
            }

            for folder in &browser_state.discovered_folders.clone() {
                let is_selected = browser_state.selected_folder == *folder;
                let depth = folder.matches('/').count();
                let display_name = folder.split('/').next_back().unwrap_or(folder);

                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 12.0);
                    if ui
                        .selectable_label(
                            is_selected,
                            egui::RichText::new(display_name).size(12.0),
                        )
                        .clicked()
                    {
                        browser_state.selected_folder = folder.clone();
                    }
                });
            }
        });
}

/// Render the asset list for the selected folder.
fn render_asset_list(
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
        egui::ScrollArea::vertical().show(ui, |ui| {
            for asset in filtered_assets {
                render_asset_row(
                    ui,
                    asset,
                    selected_asset,
                    current_tool,
                    browser_state,
                    thumbnail_cache,
                );
            }
        });
    }
}

/// Render a single asset row with thumbnail.
fn render_asset_row(
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
        .map(|a| a.relative_path == asset.relative_path)
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

        let is_missing =
            thumbnail_cache.has_failed(&asset.full_path) || !asset.full_path.exists();

        let label_text = if is_missing {
            egui::RichText::new(&asset.name).color(egui::Color32::from_rgb(200, 100, 100))
        } else {
            egui::RichText::new(&asset.name)
        };

        let response = ui.selectable_label(is_selected, label_text);
        let response = if is_missing {
            response.on_hover_text("Asset file not found")
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
fn render_selected_asset_info(
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
