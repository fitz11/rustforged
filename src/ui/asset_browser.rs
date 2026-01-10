use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle, EguiUserTextures};
use std::path::PathBuf;

use crate::assets::{
    create_and_open_library, get_image_dimensions, load_thumbnail, open_library_directory,
    AssetCategory, AssetLibrary, LibraryAsset, SelectedAsset, ThumbnailCache, THUMBNAIL_SIZE,
};
use crate::editor::{CurrentTool, EditorTool};
use crate::map::{LoadMapRequest, MapData};

use super::asset_import::AssetImportDialog;
use super::file_menu::FileMenuState;

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

            // Library management and subsections (shown when expanded)
            if browser_state.library_expanded {
                ui.add_space(4.0);

                // Library management buttons
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

                ui.add_space(8.0);

                // Maps subsection
                ui.horizontal(|ui| {
                    ui.separator();
                    ui.label(egui::RichText::new("Maps").strong());
                    ui.separator();
                });

                ui.horizontal(|ui| {
                    if ui.button("New").clicked() {
                        menu_state.show_new_confirmation = true;
                    }
                    if ui.button("Save").clicked() {
                        menu_state.save_filename = map_data.name.clone();
                        menu_state.show_save_name_dialog = true;
                    }
                    if ui.button("Load").clicked() {
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

                ui.add_space(8.0);

                // Assets subsection
                ui.horizontal(|ui| {
                    ui.separator();
                    ui.label(egui::RichText::new("Assets").strong());
                    ui.separator();
                });

                ui.horizontal(|ui| {
                    if ui.button("Import...").clicked() {
                        import_dialog.is_open = true;
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

            // =========================================
            // SELECTED ASSET METADATA SECTION
            // =========================================
            if let Some(ref asset) = selected_asset.asset {
                ui.label(egui::RichText::new("Selected Asset").strong());
                ui.add_space(4.0);

                // Asset name
                ui.label(&asset.name);

                ui.add_space(4.0);

                // Metadata in a subtle style
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Type:").small().weak());
                    ui.label(
                        egui::RichText::new(asset.extension.to_uppercase())
                            .small()
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
                    ui.label(egui::RichText::new("Size:").small().weak());
                    if let Some((width, height)) = browser_state.selected_dimensions {
                        ui.label(
                            egui::RichText::new(format!("{}x{}", width, height))
                                .small()
                                .strong(),
                        );
                    } else {
                        ui.label(egui::RichText::new("Unknown").small().weak());
                    }
                });
            } else {
                ui.label(egui::RichText::new("No asset selected").weak());
            }
        });
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
