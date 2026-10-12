//! Maps section of the asset browser: open maps, New/Open/Save, and the library's map list.

use bevy::prelude::*;
use bevy_egui::egui;
use std::path::PathBuf;

use crate::assets::AssetLibrary;
use crate::map::SwitchMapRequest;
use crate::ui::map_actions::MapFileAction;

use super::helpers::{MapTreeRow, indented, map_tree, scan_maps_directory};
use super::state::{AssetBrowserState, MapResources};

/// Render the maps subsection.
pub(super) fn render_maps_section(
    ui: &mut egui::Ui,
    library: &AssetLibrary,
    browser_state: &mut AssetBrowserState,
    map_res: &mut MapResources,
) {
    ui.label(egui::RichText::new("Maps").size(13.0).strong());
    ui.separator();

    // Show open maps with unsaved indicators
    let mut map_to_switch: Option<u64> = None;
    let mut map_to_close: Option<u64> = None;

    if !map_res.open_maps.maps.is_empty() {
        ui.label(egui::RichText::new("Open:").size(12.0).weak());

        let mut sorted_maps: Vec<_> = map_res.open_maps.maps.values().collect();
        sorted_maps.sort_by_key(|m| m.id);

        for map in sorted_maps {
            let is_active = map_res.open_maps.active_map_id == Some(map.id);
            let unsaved = map_res.open_maps.is_unsaved(map, &map_res.dirty_state);
            let display_name = if unsaved {
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

                if map.path.is_some() && unsaved {
                    ui.label(
                        egui::RichText::new("(modified)")
                            .size(10.0)
                            .weak()
                            .italics(),
                    );
                } else if map.path.is_none() {
                    ui.label(egui::RichText::new("(new)").size(10.0).weak().italics());
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("×")
                        .on_hover_text("Close this map (asks about unsaved changes)")
                        .clicked()
                    {
                        map_to_close = Some(map.id);
                    }
                });
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

    if let Some(id) = map_to_close {
        map_res.file_actions.write(MapFileAction::Close(id));
    }

    if let Some(target_id) = map_to_switch {
        map_res.switch_events.write(SwitchMapRequest {
            map_id: target_id,
            close_current: false,
        });
    }

    ui.horizontal(|ui| {
        if ui
            .add_sized([45.0, 24.0], egui::Button::new("New"))
            .on_hover_text("New map (Ctrl+N)")
            .clicked()
        {
            map_res.file_actions.write(MapFileAction::New);
        }
        if ui
            .add_sized([45.0, 24.0], egui::Button::new("Open"))
            .on_hover_text("Open a map file (Ctrl+O)")
            .clicked()
        {
            map_res
                .open_map_picker
                .open(&library.library_path.join("maps"));
        }
        if ui
            .add_sized([45.0, 24.0], egui::Button::new("Save"))
            .on_hover_text("Save map (Ctrl+S, or Ctrl+Shift+S to save as)")
            .clicked()
        {
            map_res.file_actions.write(MapFileAction::Save);
        }
        if ui
            .add_sized([55.0, 24.0], egui::Button::new("Rename"))
            .on_hover_text("Rename map (F3)")
            .clicked()
        {
            browser_state.rename_map_new_name = map_res
                .open_maps
                .active_map()
                .map(|m| m.name.clone())
                .unwrap_or_else(|| map_res.map_data.name.clone());
            browser_state.rename_map_dialog_open = true;
        }
    });

    render_disk_change_notice(ui, map_res);

    // Scan and show available maps (the folder is created by the first save)
    let maps_dir = library.library_path.join("maps");

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

        // Maps in subfolders are indented under a header per folder; sync clients'
        // conflict copies are listed under the map they conflict with
        let rows = map_tree(&browser_state.cached_maps);

        let mut rescan = false;
        egui::ScrollArea::vertical()
            .id_salt("maps_scroll")
            .max_height(120.0)
            .show(ui, |ui| {
                for row in &rows {
                    match row {
                        MapTreeRow::Folder { name, depth } => {
                            ui.horizontal(|ui| {
                                // Line up with map names, after the current-map marker
                                ui.add_space(12.0);
                                ui.label(
                                    egui::RichText::new(indented(name, *depth))
                                        .size(12.0)
                                        .weak(),
                                );
                            });
                        }
                        MapTreeRow::Map {
                            name,
                            depth,
                            path,
                            conflict_copy,
                            conflict_copies,
                        } => {
                            render_map_row(
                                ui,
                                &indented(name, *depth),
                                path,
                                *conflict_copy,
                                map_res,
                            );
                            for (copy_name, copy_path) in conflict_copies {
                                render_map_row(
                                    ui,
                                    &indented(copy_name, depth + 1),
                                    copy_path,
                                    true,
                                    map_res,
                                );
                                rescan |= discard_conflict_copy(ui, copy_path, browser_state);
                            }
                        }
                    }
                }
            });
        if rescan {
            browser_state.cached_maps = scan_maps_directory(&maps_dir);
        }
    }
}

/// One entry in the available-maps list; clicking it opens the map. Conflict copies get a
/// warning marker.
pub(super) fn render_map_row(
    ui: &mut egui::Ui,
    label: &str,
    path: &PathBuf,
    conflict_copy: bool,
    map_res: &mut MapResources,
) {
    let is_current = map_res.current_map_file.path.as_ref() == Some(path);

    ui.horizontal(|ui| {
        if is_current {
            ui.label(egui::RichText::new(">").size(10.0));
        } else {
            ui.add_space(12.0);
        }

        let mut text = egui::RichText::new(label).size(12.0);
        if is_current {
            text = text.strong();
        }
        if conflict_copy {
            text = text.color(egui::Color32::from_rgb(230, 170, 60));
            ui.label(
                egui::RichText::new("\u{26a0}")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(230, 170, 60)),
            )
            .on_hover_text(
                "Conflict copy: a sync client saved this when the map was changed on two \
                 machines at once. Open it to compare, then discard the version you don't need.",
            );
        }

        if ui
            .add(egui::Button::new(text).frame(false))
            .on_hover_text(&*path.to_string_lossy())
            .clicked()
            && !is_current
        {
            map_res
                .file_actions
                .write(MapFileAction::Open(path.clone()));
        }

        if !is_current {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("+")
                    .on_hover_text(
                        "Open alongside the map you're editing, as another scene to switch to",
                    )
                    .clicked()
                {
                    map_res
                        .file_actions
                        .write(MapFileAction::OpenAlongside(path.clone()));
                }
            });
        }
    });
}

/// "Discard" for a conflict copy, with a second click to confirm. Returns true once the
/// file is deleted.
pub(super) fn discard_conflict_copy(
    ui: &mut egui::Ui,
    path: &PathBuf,
    browser_state: &mut AssetBrowserState,
) -> bool {
    let confirming = browser_state.confirm_discard.as_ref() == Some(path);
    let mut deleted = false;
    ui.horizontal(|ui| {
        ui.add_space(36.0);
        if confirming {
            if ui
                .small_button("Delete this copy")
                .on_hover_text(&*path.to_string_lossy())
                .clicked()
            {
                browser_state.confirm_discard = None;
                match std::fs::remove_file(path) {
                    Ok(()) => {
                        info!("Discarded conflict copy {:?}", path);
                        deleted = true;
                    }
                    Err(e) => warn!("Failed to delete conflict copy {:?}: {}", path, e),
                }
            }
            if ui.small_button("Keep").clicked() {
                browser_state.confirm_discard = None;
            }
        } else if ui
            .small_button("Discard")
            .on_hover_text("Delete this conflict copy")
            .clicked()
        {
            browser_state.confirm_discard = Some(path.clone());
        }
    });
    deleted
}

/// Banner shown when the open map's file changed on disk (e.g. synced from another machine).
pub(super) fn render_disk_change_notice(ui: &mut egui::Ui, map_res: &mut MapResources) {
    let Some(notice) = map_res.disk_change.notice.clone() else {
        return;
    };
    if map_res.current_map_file.path.as_ref() != Some(&notice.path) {
        return;
    }

    let name = notice
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("map");
    let message = if notice.deleted {
        format!("\u{26a0} \"{}\" was moved or deleted on disk.", name)
    } else if let Some(who) = &notice.saved_by {
        format!("\u{26a0} \"{}\" was changed on disk by {}.", name, who)
    } else {
        format!("\u{26a0} \"{}\" was changed on disk.", name)
    };

    ui.add_space(6.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(
            egui::RichText::new(message)
                .size(12.0)
                .color(egui::Color32::from_rgb(230, 170, 60)),
        );
        ui.horizontal(|ui| {
            if !notice.deleted {
                let reload = ui.small_button("Reload");
                let reload = if map_res.dirty_state.is_dirty {
                    reload
                        .on_hover_text("Open the version on disk, discarding your unsaved changes")
                } else {
                    reload.on_hover_text("Open the version on disk")
                };
                if reload.clicked() {
                    map_res
                        .load_events
                        .write(crate::map::LoadMapRequest::replace(notice.path.clone()));
                }
            }
            if ui
                .small_button("Dismiss")
                .on_hover_text("Keep working on your version; saving will ask before overwriting")
                .clicked()
            {
                map_res.disk_change.notice = None;
            }
        });
    });
}
