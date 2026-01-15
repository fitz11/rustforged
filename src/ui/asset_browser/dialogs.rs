//! Dialog windows for the asset browser (rename, move, import errors, etc.).

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::assets::{
    AssetLibrary, RenameAssetRequest, SelectedAsset, ThumbnailCache,
    UpdateLibraryMetadataRequest,
};
use crate::config::SetDefaultLibraryRequest;

use super::asset_ops::{move_asset, rename_asset};
use super::helpers::discover_folders;
use super::state::{AssetBrowserState, MapResources};

/// Render the "Set as default library" dialog.
pub fn render_set_default_dialog(
    ctx: &egui::Context,
    browser_state: &mut AssetBrowserState,
    set_default_events: &mut MessageWriter<SetDefaultLibraryRequest>,
) {
    if !browser_state.show_set_default_dialog {
        return;
    }

    egui::Window::new("Library Opened")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
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
                    set_default_events.write(SetDefaultLibraryRequest { path: path.clone() });
                }
                browser_state.show_set_default_dialog = false;
                browser_state.set_default_dialog_path = None;
                browser_state.set_as_default_checked = false;
            }
        });
}

/// Render the library import error dialog.
pub fn render_import_error_dialog(ctx: &egui::Context, browser_state: &mut AssetBrowserState) {
    let Some(ref error) = browser_state.library_import_error.clone() else {
        return;
    };

    egui::Window::new("Import Error")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.colored_label(egui::Color32::RED, "Failed to import library");
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    ui.label(error);
                });
            ui.add_space(8.0);
            if ui.button("OK").clicked() {
                browser_state.library_import_error = None;
            }
        });
}

/// Render the library operation success dialog.
pub fn render_success_dialog(ctx: &egui::Context, browser_state: &mut AssetBrowserState) {
    let Some(ref message) = browser_state.library_operation_success.clone() else {
        return;
    };

    egui::Window::new("Success")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            if ui.button("OK").clicked() {
                browser_state.library_operation_success = None;
            }
        });
}

/// Render the asset rename dialog.
pub fn render_rename_asset_dialog(
    ctx: &egui::Context,
    browser_state: &mut AssetBrowserState,
    selected_asset: &mut SelectedAsset,
    library: &mut AssetLibrary,
    thumbnail_cache: &mut ThumbnailCache,
    rename_events: &mut MessageWriter<RenameAssetRequest>,
) {
    if !browser_state.rename_dialog_open {
        return;
    }

    let mut close_dialog = false;
    let mut do_rename = false;

    egui::Window::new("Rename Asset")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Enter new name:");
            ui.add_space(4.0);

            let response = ui.text_edit_singleline(&mut browser_state.rename_new_name);

            // Request focus only when first opened (not every frame)
            if !response.has_focus() {
                response.request_focus();
            }

            // Handle Enter key
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_rename = true;
            }

            // Show error message if any
            if let Some(ref error) = browser_state.rename_error {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Rename").clicked() {
                    do_rename = true;
                }
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    // Handle rename action
    if do_rename
        && let Some(ref asset) = selected_asset.asset.clone()
    {
        match rename_asset(
            &asset.full_path,
            &browser_state.rename_new_name,
            &library.library_path,
        ) {
            Ok((new_path, old_relative, new_relative)) => {
                // Update the asset in the library
                if let Some(lib_asset) = library
                    .assets
                    .iter_mut()
                    .find(|a| a.full_path == asset.full_path)
                {
                    lib_asset.full_path = new_path.clone();
                    lib_asset.relative_path = new_relative.clone();
                    lib_asset.name = browser_state.rename_new_name.trim().to_string();
                }

                // Update the selected asset
                if let Some(ref mut sel) = selected_asset.asset {
                    sel.full_path = new_path;
                    sel.relative_path = new_relative.clone();
                    sel.name = browser_state.rename_new_name.trim().to_string();
                }

                // Clear thumbnail cache for this asset (path changed)
                thumbnail_cache.thumbnails.remove(&asset.full_path);
                thumbnail_cache.texture_ids.remove(&asset.full_path);

                // Emit message to update currently placed items
                rename_events.write(RenameAssetRequest {
                    old_path: old_relative,
                    new_path: new_relative,
                });

                browser_state.rename_dialog_open = false;
                browser_state.rename_error = None;
                info!(
                    "Renamed asset: {} -> {}",
                    asset.name, browser_state.rename_new_name
                );
            }
            Err(e) => {
                browser_state.rename_error = Some(e);
            }
        }
    }

    if close_dialog {
        browser_state.rename_dialog_open = false;
        browser_state.rename_error = None;
    }
}

/// Render the rename map dialog.
pub fn render_rename_map_dialog(
    ctx: &egui::Context,
    browser_state: &mut AssetBrowserState,
    map_res: &mut MapResources,
) {
    if !browser_state.rename_map_dialog_open {
        return;
    }

    let mut close_dialog = false;
    let mut do_rename = false;

    egui::Window::new("Rename Map")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Enter new map name:");
            ui.add_space(4.0);

            let response = ui.text_edit_singleline(&mut browser_state.rename_map_new_name);

            // Request focus only when first opened (not every frame)
            if !response.has_focus() {
                response.request_focus();
            }

            // Handle Enter key
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_rename = true;
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Rename").clicked() {
                    do_rename = true;
                }
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if do_rename {
        let new_name = browser_state.rename_map_new_name.trim().to_string();
        if !new_name.is_empty() {
            map_res.map_data.name = new_name;
            map_res.dirty_state.is_dirty = true;
            browser_state.rename_map_dialog_open = false;
        }
    }

    if close_dialog {
        browser_state.rename_map_dialog_open = false;
    }
}

/// Render the rename library dialog.
pub fn render_rename_library_dialog(
    ctx: &egui::Context,
    browser_state: &mut AssetBrowserState,
    library_metadata_events: &mut MessageWriter<UpdateLibraryMetadataRequest>,
) {
    if !browser_state.rename_library_dialog_open {
        return;
    }

    let mut close_dialog = false;
    let mut do_rename = false;

    egui::Window::new("Rename Library")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Enter new library name:");
            ui.add_space(4.0);

            let response = ui.text_edit_singleline(&mut browser_state.rename_library_new_name);

            // Request focus only when first opened (not every frame)
            if !response.has_focus() {
                response.request_focus();
            }

            // Handle Enter key
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_rename = true;
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Rename").clicked() {
                    do_rename = true;
                }
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if do_rename {
        let new_name = browser_state.rename_library_new_name.trim().to_string();
        if !new_name.is_empty() {
            library_metadata_events.write(UpdateLibraryMetadataRequest { name: new_name });
            browser_state.rename_library_dialog_open = false;
        }
    }

    if close_dialog {
        browser_state.rename_library_dialog_open = false;
    }
}

/// Render the move asset dialog.
pub fn render_move_asset_dialog(
    ctx: &egui::Context,
    browser_state: &mut AssetBrowserState,
    selected_asset: &mut SelectedAsset,
    library: &mut AssetLibrary,
    thumbnail_cache: &mut ThumbnailCache,
    rename_events: &mut MessageWriter<RenameAssetRequest>,
) {
    if !browser_state.move_dialog_open {
        return;
    }

    let mut close_dialog = false;
    let mut target_folder: Option<String> = None;

    egui::Window::new("Move Asset")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some(ref asset) = selected_asset.asset {
                ui.label(format!("Move '{}' to:", asset.name));
                ui.add_space(8.0);

                // Root folder option
                if !asset.folder_path.is_empty() && ui.button("(root)").clicked() {
                    target_folder = Some(String::new());
                }

                // Existing folder options (excluding current folder)
                for folder in &browser_state.discovered_folders.clone() {
                    if *folder != asset.folder_path && ui.button(folder).clicked() {
                        target_folder = Some(folder.clone());
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.label("Or create new folder:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut browser_state.move_new_folder_name);
                    if ui.button("Create & Move").clicked()
                        && !browser_state.move_new_folder_name.is_empty()
                    {
                        target_folder = Some(browser_state.move_new_folder_name.clone());
                    }
                });

                // Show error message if any
                if let Some(ref error) = browser_state.move_error {
                    ui.add_space(4.0);
                    ui.colored_label(egui::Color32::RED, error);
                }

                ui.add_space(8.0);
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            } else {
                ui.label("No asset selected");
                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            }
        });

    // Handle move action
    if let Some(folder) = target_folder
        && let Some(ref asset) = selected_asset.asset.clone()
    {
        match move_asset(&asset.full_path, &folder, &library.library_path) {
            Ok((new_path, old_relative, new_relative)) => {
                // Update the asset in the library
                if let Some(lib_asset) = library
                    .assets
                    .iter_mut()
                    .find(|a| a.full_path == asset.full_path)
                {
                    lib_asset.full_path = new_path.clone();
                    lib_asset.relative_path = new_relative.clone();
                    lib_asset.folder_path = folder.clone();
                }

                // Update the selected asset
                if let Some(ref mut sel) = selected_asset.asset {
                    sel.full_path = new_path;
                    sel.relative_path = new_relative.clone();
                    sel.folder_path = folder.clone();
                }

                // Update the browser to show the new folder
                browser_state.selected_folder = folder.clone();
                // Refresh discovered folders
                browser_state.discovered_folders = discover_folders(library);

                // Clear thumbnail cache for this asset (path changed)
                thumbnail_cache.thumbnails.remove(&asset.full_path);
                thumbnail_cache.texture_ids.remove(&asset.full_path);

                // Emit message to update currently placed items
                rename_events.write(RenameAssetRequest {
                    old_path: old_relative,
                    new_path: new_relative,
                });

                browser_state.move_dialog_open = false;
                browser_state.move_error = None;
                browser_state.move_new_folder_name.clear();
                let folder_display = if folder.is_empty() {
                    "(root)"
                } else {
                    &folder
                };
                info!("Moved asset '{}' to {}", asset.name, folder_display);
            }
            Err(e) => {
                browser_state.move_error = Some(e);
            }
        }
    }

    if close_dialog {
        browser_state.move_dialog_open = false;
        browser_state.move_error = None;
        browser_state.move_new_folder_name.clear();
    }
}

/// Handle keyboard shortcuts for opening rename dialogs.
pub fn handle_rename_shortcuts(
    contexts: &mut EguiContexts,
    browser_state: &mut AssetBrowserState,
    selected_asset: &SelectedAsset,
    map_res: &MapResources,
    library: &AssetLibrary,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        // Handle F2 key to open rename dialog for selected asset
        if ctx.input(|i| i.key_pressed(egui::Key::F2))
            && !browser_state.rename_dialog_open
            && let Some(ref asset) = selected_asset.asset
        {
            browser_state.rename_new_name = asset.name.clone();
            browser_state.rename_error = None;
            browser_state.rename_dialog_open = true;
        }

        // Handle F3 key to open rename map dialog
        if ctx.input(|i| i.key_pressed(egui::Key::F3)) && !browser_state.rename_map_dialog_open {
            browser_state.rename_map_new_name = map_res.map_data.name.clone();
            browser_state.rename_map_dialog_open = true;
        }

        // Handle F4 key to open rename library dialog
        if ctx.input(|i| i.key_pressed(egui::Key::F4))
            && !browser_state.rename_library_dialog_open
        {
            browser_state.rename_library_new_name = library.metadata.name.clone();
            browser_state.rename_library_dialog_open = true;
        }

        // Handle Escape to close rename dialogs
        if browser_state.rename_dialog_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            browser_state.rename_dialog_open = false;
            browser_state.rename_error = None;
        }
        if browser_state.rename_map_dialog_open && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            browser_state.rename_map_dialog_open = false;
        }
        if browser_state.rename_library_dialog_open
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            browser_state.rename_library_dialog_open = false;
        }
    }
}
