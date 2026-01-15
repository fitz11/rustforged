//! Asset browser state resources and SystemParams.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::path::PathBuf;

use crate::map::{
    CurrentMapFile, LoadMapRequest, MapData, MapDirtyState, OpenMaps, SwitchMapRequest,
};

use super::super::file_menu::FileMenuState;
use super::super::asset_import::AssetImportDialog;
use super::super::settings_dialog::SettingsDialogState;

/// Bundle of map-related resources and event writers.
#[derive(SystemParam)]
pub struct MapResources<'w> {
    pub map_data: ResMut<'w, MapData>,
    pub current_map_file: Res<'w, CurrentMapFile>,
    pub dirty_state: ResMut<'w, MapDirtyState>,
    pub open_maps: Res<'w, OpenMaps>,
    pub load_events: MessageWriter<'w, LoadMapRequest>,
    pub switch_events: MessageWriter<'w, SwitchMapRequest>,
}

/// Bundle of dialog state resources.
#[derive(SystemParam)]
pub struct DialogStates<'w> {
    pub menu_state: ResMut<'w, FileMenuState>,
    pub import_dialog: ResMut<'w, AssetImportDialog>,
    pub settings_state: ResMut<'w, SettingsDialogState>,
}

/// State resource for the asset browser panel.
#[derive(Resource)]
pub struct AssetBrowserState {
    /// Currently selected folder path (empty string = root)
    pub selected_folder: String,
    /// Cached list of discovered folders in the library
    pub discovered_folders: Vec<String>,
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
    /// Error message for library import failures
    pub library_import_error: Option<String>,
    /// Success message for export/import operations
    pub library_operation_success: Option<String>,
    /// Whether the rename dialog is open
    pub rename_dialog_open: bool,
    /// New name input for rename dialog
    pub rename_new_name: String,
    /// Error message for rename operation
    pub rename_error: Option<String>,
    /// Whether the rename map dialog is open
    pub rename_map_dialog_open: bool,
    /// New name input for rename map dialog
    pub rename_map_new_name: String,
    /// Whether the rename library dialog is open
    pub rename_library_dialog_open: bool,
    /// New name input for rename library dialog
    pub rename_library_new_name: String,
    /// Whether the move asset dialog is open
    pub move_dialog_open: bool,
    /// Error message for move operation
    pub move_error: Option<String>,
    /// New folder name input for move dialog
    pub move_new_folder_name: String,
}

impl Default for AssetBrowserState {
    fn default() -> Self {
        Self {
            selected_folder: String::new(),
            discovered_folders: Vec::new(),
            library_expanded: true,
            selected_dimensions: None,
            cached_dimensions_path: None,
            last_library_path: None,
            show_set_default_dialog: false,
            set_default_dialog_path: None,
            set_as_default_checked: false,
            cached_maps: Vec::new(),
            last_maps_scan_path: None,
            library_import_error: None,
            library_operation_success: None,
            rename_dialog_open: false,
            rename_new_name: String::new(),
            rename_error: None,
            rename_map_dialog_open: false,
            rename_map_new_name: String::new(),
            rename_library_dialog_open: false,
            rename_library_new_name: String::new(),
            move_dialog_open: false,
            move_error: None,
            move_new_folder_name: String::new(),
        }
    }
}
