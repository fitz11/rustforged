use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle, EguiUserTextures};
use std::path::PathBuf;

use crate::assets::{
    create_and_open_library, get_image_dimensions, load_thumbnail, open_library_directory,
    AssetCategory, AssetLibrary, LibraryAsset, SelectedAsset, ThumbnailCache, THUMBNAIL_SIZE,
};
use crate::config::{AppConfig, SetDefaultLibrary};
use crate::editor::{CurrentTool, EditorTool};
use crate::map::{CurrentMapFile, LoadMapRequest, MapData, MapDirtyState, OpenMaps, SwitchMapRequest};

use super::asset_import::AssetImportDialog;
use super::file_menu::FileMenuState;

/// Scan the maps directory and return sorted list of map names (without extension)
fn scan_maps_directory(maps_dir: &std::path::Path) -> Vec<(String, PathBuf)> {
    if !maps_dir.exists() {
        return Vec::new();
    }

    let mut maps: Vec<(String, PathBuf)> = std::fs::read_dir(maps_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension().and_then(|ext| ext.to_str()) == Some("json")
                })
                .map(|e| {
                    let path = e.path();
                    let name = path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    (name, path)
                })
                .collect()
        })
        .unwrap_or_default();

    maps.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    maps
}

/// System that loads thumbnails and registers them with egui.
/// Runs in Update before the egui pass to avoid timing issues.
pub fn load_and_register_thumbnails(
    library: Res<AssetLibrary>,
    mut thumbnail_cache: ResMut<ThumbnailCache>,
    mut images: ResMut<Assets<Image>>,
    mut egui_textures: ResMut<EguiUserTextures>,
) {
    // Load up to 3 new thumbnails per frame
    let assets_to_load: Vec<PathBuf> = library
        .assets
        .iter()
        .filter(|a| {
            !thumbnail_cache.thumbnails.contains_key(&a.full_path)
                && !thumbnail_cache.has_failed(&a.full_path)
        })
        .take(3)
        .map(|a| a.full_path.clone())
        .collect();

    for path in assets_to_load {
        if let Some(thumb_image) = load_thumbnail(&path) {
            let handle = images.add(thumb_image);
            thumbnail_cache.thumbnails.insert(path, handle);
        } else {
            thumbnail_cache.failed.insert(path, ());
        }
    }

    // Register any thumbnails that don't have texture IDs yet
    let to_register: Vec<PathBuf> = thumbnail_cache
        .thumbnails
        .keys()
        .filter(|path| !thumbnail_cache.texture_ids.contains_key(*path))
        .cloned()
        .collect();

    for path in to_register {
        if let Some(handle) = thumbnail_cache.thumbnails.get(&path) {
            let texture_id = egui_textures.add_image(EguiTextureHandle::Weak(handle.id()));
            thumbnail_cache.texture_ids.insert(path, texture_id);
        }
    }
}

#[derive(Resource, Default)]
pub struct AssetBrowserState {
    pub selected_category: AssetCategory,
    /// Whether the library info section is expanded
    pub library_expanded: bool,
    /// Cached dimensions for the selected asset
    pub selected_dimensions: Option<(u32, u32)>,
    /// Path of the asset for which dimensions are cached
    pub cached_dimensions_path: Option<PathBuf>,
    /// Last known library path (to detect changes and clear thumbnail cache)
    pub last_library_path: Option<PathBuf>,
    /// Whether the "set as default" dialog is shown
    pub show_set_default_dialog: bool,
    /// Path for the "set as default" dialog
    pub set_default_dialog_path: Option<PathBuf>,
    /// Checkbox state for "set as default" dialog
    pub set_as_default_checked: bool,
    /// Cached list of available maps in the library
    pub cached_maps: Vec<(String, PathBuf)>,
    /// Last path used for map scanning (to detect changes)
    pub last_maps_scan_path: Option<PathBuf>,
}

#[allow(clippy::too_many_arguments)]
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
    mut thumbnail_cache: ResMut<ThumbnailCache>,
    config: Res<AppConfig>,
    mut set_default_events: MessageWriter<SetDefaultLibrary>,
    current_map_file: Res<CurrentMapFile>,
    dirty_state: Res<MapDirtyState>,
    open_maps: Res<OpenMaps>,
    mut switch_events: MessageWriter<SwitchMapRequest>,
) -> Result {
    // Clear thumbnail cache if library path changed
    let current_path = library.library_path.clone();
    if browser_state.last_library_path.as_ref() != Some(&current_path) {
        thumbnail_cache.clear();
        browser_state.last_library_path = Some(current_path);
    }

    egui::SidePanel::left("asset_browser")
        .default_width(220.0)
        .show(contexts.ctx_mut()?, |ui| {
            // =========================================
            // ASSET LIBRARY SECTION
            // =========================================
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
                ui.label(egui::RichText::new("Asset Library").heading().size(18.0));
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

                // Library management buttons
                ui.horizontal(|ui| {
                    if ui.add_sized([70.0, 24.0], egui::Button::new("Open...")).clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .set_title("Open Asset Library")
                            .pick_folder()
                    {
                        // Try to open the library
                        if let Err(e) = open_library_directory(&mut library, path.clone()) {
                            warn!("Failed to open library: {}", e);
                        } else {
                            // Show the "set as default" dialog
                            browser_state.show_set_default_dialog = true;
                            browser_state.set_default_dialog_path = Some(path);
                            browser_state.set_as_default_checked = false;
                        }
                    }

                    if ui.add_sized([70.0, 24.0], egui::Button::new("New...")).clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .set_title("Create New Asset Library")
                            .pick_folder()
                    {
                        if let Err(e) = create_and_open_library(&mut library, path.clone()) {
                            warn!("Failed to create library: {}", e);
                        } else {
                            // Show the "set as default" dialog
                            browser_state.show_set_default_dialog = true;
                            browser_state.set_default_dialog_path = Some(path);
                            browser_state.set_as_default_checked = false;
                        }
                    }
                });

                // Recent libraries section
                if !config.data.recent_libraries.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Recent:").small().weak());
                    for recent_path in &config.data.recent_libraries {
                        // Skip current library
                        if recent_path == &library.library_path {
                            continue;
                        }
                        // Skip if doesn't exist
                        if !recent_path.exists() {
                            continue;
                        }
                        let display_name = recent_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        let full_path_str = recent_path.to_string_lossy();
                        if ui
                            .small_button(display_name)
                            .on_hover_text(full_path_str.as_ref())
                            .clicked()
                            && let Err(e) = open_library_directory(&mut library, recent_path.clone())
                        {
                            warn!("Failed to open recent library: {}", e);
                        }
                    }
                }

                ui.add_space(10.0);

                // Maps subsection
                ui.horizontal(|ui| {
                    ui.separator();
                    ui.label(egui::RichText::new("Maps").size(14.0).strong());
                    ui.separator();
                });
                ui.add_space(4.0);

                // Show open maps with unsaved indicators
                let mut map_to_switch: Option<u64> = None;

                if !open_maps.maps.is_empty() {
                    ui.label(egui::RichText::new("Open:").size(12.0).weak());

                    // Sort maps by ID to maintain consistent order
                    let mut sorted_maps: Vec<_> = open_maps.maps.values().collect();
                    sorted_maps.sort_by_key(|m| m.id);

                    for map in sorted_maps {
                        let is_active = open_maps.active_map_id == Some(map.id);
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

                            // Make non-active maps clickable
                            if is_active {
                                ui.label(text);
                            } else if ui
                                .add(egui::Button::new(text).frame(false))
                                .on_hover_text("Click to switch to this map")
                                .clicked()
                            {
                                map_to_switch = Some(map.id);
                            }

                            // Show path hint if saved
                            if map.path.is_some() && map.is_dirty {
                                ui.label(egui::RichText::new("(modified)").size(10.0).weak().italics());
                            } else if map.path.is_none() {
                                ui.label(egui::RichText::new("(new)").size(10.0).weak().italics());
                            }
                        });
                    }
                    ui.add_space(4.0);
                } else {
                    // Fallback to showing current map indicator
                    if let Some(ref current_path) = current_map_file.path {
                        let current_name = current_path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        let dirty_indicator = if dirty_state.is_dirty { "*" } else { "" };
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
                        let dirty_indicator = if dirty_state.is_dirty { "*" } else { "" };
                        ui.label(
                            egui::RichText::new(format!("(unsaved map){}", dirty_indicator))
                                .size(12.0)
                                .weak()
                                .italics(),
                        );
                        ui.add_space(4.0);
                    }
                }

                // Switch to selected map if requested
                if let Some(target_id) = map_to_switch {
                    switch_events.write(SwitchMapRequest { map_id: target_id });
                }

                ui.horizontal(|ui| {
                    if ui.add_sized([50.0, 24.0], egui::Button::new("New")).clicked() {
                        menu_state.show_new_confirmation = true;
                    }
                    if ui.add_sized([50.0, 24.0], egui::Button::new("Save")).clicked() {
                        menu_state.save_filename = map_data.name.clone();
                        menu_state.show_save_name_dialog = true;
                    }
                    if ui.add_sized([50.0, 24.0], egui::Button::new("Load")).clicked() {
                        let maps_dir = library.library_path.join("maps");
                        let maps_dir = if maps_dir.exists() {
                            maps_dir
                        } else {
                            PathBuf::from("assets/maps")
                        };
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Map Files", &["json"])
                            .set_directory(&maps_dir)
                            .set_title("Load Map")
                            .pick_file()
                        {
                            load_events.write(LoadMapRequest { path });
                        }
                    }
                });

                // Scan maps directory and show available maps
                let maps_dir = library.library_path.join("maps");
                let fallback_maps_dir = PathBuf::from("assets/maps");
                let effective_maps_dir = if maps_dir.exists() {
                    maps_dir
                } else {
                    fallback_maps_dir
                };

                // Refresh cached maps if directory changed
                if browser_state.last_maps_scan_path.as_ref() != Some(&effective_maps_dir) {
                    browser_state.cached_maps = scan_maps_directory(&effective_maps_dir);
                    browser_state.last_maps_scan_path = Some(effective_maps_dir.clone());
                }

                if !browser_state.cached_maps.is_empty() {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Available:").size(12.0).weak());
                        if ui.small_button("R").on_hover_text("Refresh map list").clicked() {
                            browser_state.cached_maps = scan_maps_directory(&effective_maps_dir);
                        }
                    });

                    // Show map buttons in a scrollable area
                    egui::ScrollArea::vertical()
                        .id_salt("maps_scroll")
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for (map_name, map_path) in &browser_state.cached_maps {
                                let is_current = current_map_file
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
                                        load_events.write(LoadMapRequest {
                                            path: map_path.clone(),
                                        });
                                    }
                                });
                            }
                        });
                }

                ui.add_space(10.0);

                // Assets subsection
                ui.horizontal(|ui| {
                    ui.separator();
                    ui.label(egui::RichText::new("Assets").size(14.0).strong());
                    ui.separator();
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.add_sized([80.0, 24.0], egui::Button::new("Import...")).clicked() {
                        import_dialog.is_open = true;
                    }
                });

                ui.add_space(6.0);
            }

            ui.separator();
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                for category in AssetCategory::all() {
                    let selected = browser_state.selected_category == *category;
                    if ui
                        .selectable_label(selected, egui::RichText::new(category.display_name()).size(13.0))
                        .clicked()
                    {
                        browser_state.selected_category = *category;
                    }
                }
            });

            ui.add_space(4.0);
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

                        // Asset row with thumbnail preview
                        ui.horizontal(|ui| {
                            let thumb_size = THUMBNAIL_SIZE as f32;

                            // Try to get cached thumbnail texture ID
                            if let Some(texture_id) =
                                thumbnail_cache.get_texture_id(&asset.full_path)
                            {
                                ui.add(
                                    egui::Image::new(egui::load::SizedTexture::new(
                                        texture_id,
                                        egui::vec2(thumb_size, thumb_size),
                                    ))
                                    .fit_to_exact_size(egui::vec2(thumb_size, thumb_size))
                                    .corner_radius(2.0),
                                );
                            } else {
                                // Placeholder while loading
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(thumb_size, thumb_size),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    2.0,
                                    egui::Color32::from_rgb(60, 60, 60),
                                );
                            }

                            // Asset name (selectable)
                            if ui
                                .selectable_label(is_selected, &asset.name)
                                .clicked()
                            {
                                browser_state.selected_dimensions = None;
                                browser_state.cached_dimensions_path = None;
                                selected_asset.asset = Some(asset.clone());
                                current_tool.tool = EditorTool::Place;
                            }

                            // File type badge on the right
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
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
                                },
                            );
                        });
                    }
                });
            }

            ui.separator();
            ui.add_space(4.0);

            // =========================================
            // SELECTED ASSET METADATA SECTION
            // =========================================
            if let Some(ref asset) = selected_asset.asset {
                ui.label(egui::RichText::new("Selected Asset").size(14.0).strong());
                ui.add_space(6.0);

                // Asset name
                ui.label(egui::RichText::new(&asset.name).size(13.0));

                ui.add_space(6.0);

                // Metadata in a subtle style
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Type:").size(13.0).weak());
                    ui.label(
                        egui::RichText::new(asset.extension.to_uppercase())
                            .size(13.0)
                            .strong(),
                    );
                });

                // Get/cache dimensions
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
        });

    // "Set as default library" dialog
    if browser_state.show_set_default_dialog {
        egui::Window::new("Library Opened")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(contexts.ctx_mut()?, |ui| {
                if let Some(ref path) = browser_state.set_default_dialog_path {
                    let display_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown");
                    ui.label(format!("Opened library: {}", display_name));
                    ui.add_space(8.0);
                    ui.checkbox(
                        &mut browser_state.set_as_default_checked,
                        "Set as default library",
                    );
                    ui.add_space(8.0);
                }

                if ui.button("OK").clicked() {
                    if browser_state.set_as_default_checked
                        && let Some(ref path) = browser_state.set_default_dialog_path
                    {
                        set_default_events.write(SetDefaultLibrary { path: path.clone() });
                    }
                    browser_state.show_set_default_dialog = false;
                    browser_state.set_default_dialog_path = None;
                    browser_state.set_as_default_checked = false;
                }
            });
    }

    Ok(())
}

/// Get a color for the preview square based on file extension
fn extension_color(ext: &str) -> egui::Color32 {
    match ext {
        "png" => egui::Color32::from_rgb(80, 140, 200),   // Blue
        "jpg" | "jpeg" => egui::Color32::from_rgb(200, 140, 80), // Orange
        "webp" => egui::Color32::from_rgb(140, 200, 80),  // Green
        "gif" => egui::Color32::from_rgb(200, 80, 140),   // Pink
        "bmp" => egui::Color32::from_rgb(140, 80, 200),   // Purple
        "tiff" | "tif" => egui::Color32::from_rgb(80, 200, 140), // Teal
        _ => egui::Color32::from_rgb(128, 128, 128),      // Gray
    }
}
