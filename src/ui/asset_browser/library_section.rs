//! Library section of the asset browser: open, create, refresh, export and import.

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use bevy_egui::egui;

use crate::assets::{
    AssetLibrary, ThumbnailCache, create_and_open_library, open_library_directory,
};

use super::assets_section::render_assets_buttons;
use super::library_ops::{export_library_to_zip, import_destination, import_library_from_zip};
use super::maps_section::render_maps_section;
use super::state::{AssetBrowserState, DialogStates, MapResources, ZipJob, ZipJobKind};
use crate::common::poll_task;

/// Poll all pending async file dialog tasks and handle their results.
pub(super) fn poll_file_dialog_tasks(
    library: &mut AssetLibrary,
    browser_state: &mut AssetBrowserState,
) {
    // Poll: Open Library
    if let Some(ref mut task) = browser_state.pending_open_library
        && let Some(result) = poll_task(task)
    {
        browser_state.pending_open_library = None;
        if let Some(path) = result {
            if let Err(e) = open_library_directory(library, path.clone()) {
                warn!("Failed to open library: {}", e);
            } else {
                browser_state.show_set_default_dialog = true;
                browser_state.set_default_dialog_path = Some(path);
                browser_state.set_as_default_checked = false;
            }
        }
    }

    // Poll: Create Library
    if let Some(ref mut task) = browser_state.pending_create_library
        && let Some(result) = poll_task(task)
    {
        browser_state.pending_create_library = None;
        if let Some(path) = result {
            if let Err(e) = create_and_open_library(library, path.clone()) {
                warn!("Failed to create library: {}", e);
            } else {
                browser_state.show_set_default_dialog = true;
                browser_state.set_default_dialog_path = Some(path);
                browser_state.set_as_default_checked = false;
            }
        }
    }

    // Poll: Export Library (destination picked: export in the background)
    if let Some(ref mut task) = browser_state.pending_export
        && let Some(result) = poll_task(task)
    {
        browser_state.pending_export = None;
        if let Some(dest_path) = result {
            let library_path = library.library_path.clone();
            browser_state.zip_job = Some(ZipJob {
                kind: ZipJobKind::Export,
                task: IoTaskPool::get().spawn(async move {
                    export_library_to_zip(&library_path, &dest_path).map(|()| dest_path)
                }),
            });
        }
    }

    // Poll: Import Phase 1 (pick zip)
    if let Some(ref mut task) = browser_state.pending_import_zip
        && let Some(result) = poll_task(task)
    {
        browser_state.pending_import_zip = None;
        if let Some(zip_path) = result {
            // Store the zip path and spawn phase 2 dialog
            browser_state.pending_import_zip_path = Some(zip_path);
            let task_pool = IoTaskPool::get();
            browser_state.pending_import_dest = Some(task_pool.spawn(async {
                rfd::AsyncFileDialog::new()
                    .set_title("Select Destination Folder")
                    .pick_folder()
                    .await
                    .map(|h| h.path().to_path_buf())
            }));
        }
    }

    // Poll: Import Phase 2 (destination picked: extract in the background)
    if let Some(ref mut task) = browser_state.pending_import_dest
        && let Some(result) = poll_task(task)
    {
        browser_state.pending_import_dest = None;
        let zip_path = browser_state.pending_import_zip_path.take();
        if let (Some(zip_path), Some(picked)) = (zip_path, result) {
            browser_state.zip_job = Some(ZipJob {
                kind: ZipJobKind::Import,
                task: IoTaskPool::get().spawn(async move {
                    let dest = import_destination(&zip_path, &picked)?;
                    import_library_from_zip(&zip_path, &dest)?;
                    Ok(dest)
                }),
            });
        }
    }

    // Poll: export or import finished
    if let Some(job) = browser_state.zip_job.as_mut()
        && let Some(result) = poll_task(&mut job.task)
    {
        let kind = job.kind;
        browser_state.zip_job = None;
        match (kind, result) {
            (ZipJobKind::Export, Ok(zip_path)) => {
                browser_state.library_operation_success =
                    Some(format!("Library exported to:\n{}", zip_path.display()));
            }
            (ZipJobKind::Import, Ok(dest_path)) => {
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
            (_, Err(e)) => browser_state.library_import_error = Some(e),
        }
    }
}

/// Render the library management section.
pub(super) fn render_library_section(
    ui: &mut egui::Ui,
    library: &mut AssetLibrary,
    browser_state: &mut AssetBrowserState,
    dialogs: &mut DialogStates,
    map_res: &mut MapResources,
    _thumbnail_cache: &mut ThumbnailCache,
) {
    // Poll pending async file dialog tasks before rendering buttons
    poll_file_dialog_tasks(library, browser_state);

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
        ui.label(
            egui::RichText::new(&library.metadata.name)
                .heading()
                .size(18.0),
        );
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
        .on_hover_text(&*path_str);

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
        render_maps_section(ui, library, browser_state, map_res);
        ui.add_space(10.0);
        render_assets_buttons(ui, library, dialogs);
        ui.add_space(6.0);
    }
}

/// Render library management buttons (Open, Refresh, New, Rename).
pub(super) fn render_library_buttons(
    ui: &mut egui::Ui,
    library: &mut AssetLibrary,
    browser_state: &mut AssetBrowserState,
) {
    ui.horizontal(|ui| {
        if ui
            .add_sized([50.0, 24.0], egui::Button::new("Open"))
            .on_hover_text("Open existing library folder")
            .clicked()
            && browser_state.pending_open_library.is_none()
        {
            let task_pool = IoTaskPool::get();
            browser_state.pending_open_library = Some(task_pool.spawn(async {
                rfd::AsyncFileDialog::new()
                    .set_title("Open Asset Library")
                    .pick_folder()
                    .await
                    .map(|h| h.path().to_path_buf())
            }));
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
            && browser_state.pending_create_library.is_none()
        {
            let task_pool = IoTaskPool::get();
            browser_state.pending_create_library = Some(task_pool.spawn(async {
                rfd::AsyncFileDialog::new()
                    .set_title("Create New Asset Library")
                    .pick_folder()
                    .await
                    .map(|h| h.path().to_path_buf())
            }));
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
pub(super) fn render_export_import_buttons(
    ui: &mut egui::Ui,
    library: &mut AssetLibrary,
    browser_state: &mut AssetBrowserState,
) {
    ui.horizontal(|ui| {
        if ui
            .add_sized([70.0, 24.0], egui::Button::new("Export"))
            .clicked()
            && browser_state.pending_export.is_none()
        {
            let file_name = format!("{}.zip", library.metadata.name);
            let task_pool = IoTaskPool::get();
            browser_state.pending_export = Some(task_pool.spawn(async move {
                rfd::AsyncFileDialog::new()
                    .set_title("Export Library as Zip")
                    .set_file_name(file_name)
                    .add_filter("Zip Archive", &["zip"])
                    .save_file()
                    .await
                    .map(|h| h.path().to_path_buf())
            }));
        }

        if ui
            .add_sized([70.0, 24.0], egui::Button::new("Import"))
            .clicked()
            && browser_state.pending_import_zip.is_none()
            && browser_state.pending_import_dest.is_none()
        {
            let task_pool = IoTaskPool::get();
            browser_state.pending_import_zip = Some(task_pool.spawn(async {
                rfd::AsyncFileDialog::new()
                    .set_title("Import Library from Zip")
                    .add_filter("Zip Archive", &["zip"])
                    .pick_file()
                    .await
                    .map(|h| h.path().to_path_buf())
            }));
        }
    });
}
