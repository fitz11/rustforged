use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::constants::MAX_RECENT_LIBRARIES;

/// System set for config loading (other plugins can run after this)
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigLoaded;

/// Application configuration persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfigData {
    /// Default asset library path (opened on startup, only changes when user explicitly sets it)
    #[serde(default)]
    pub default_library_path: Option<PathBuf>,

    /// Recently opened asset libraries for quick access
    #[serde(default)]
    pub recent_libraries: Vec<PathBuf>,

    /// Last opened map file path (not auto-loaded, just remembered for quick access)
    #[serde(default)]
    pub last_map_path: Option<PathBuf>,
}

/// Runtime configuration resource
#[derive(Resource)]
pub struct AppConfig {
    /// The persisted configuration data
    pub data: AppConfigData,
    /// Path to the config file
    pub config_path: PathBuf,
    /// Whether config needs to be saved (dirty flag)
    pub dirty: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data: AppConfigData::default(),
            config_path: get_config_path(),
            dirty: false,
        }
    }
}

/// Resource for the "map file missing" warning dialog
#[derive(Resource, Default)]
pub struct MissingMapWarning {
    pub show: bool,
    pub path: Option<PathBuf>,
}

/// Resource to notify user when config was reset to defaults
#[derive(Resource, Default)]
pub struct ConfigResetNotification {
    /// Whether to show the notification dialog
    pub show: bool,
    /// The reason for the reset (parse error, read error, etc.)
    pub reason: Option<String>,
}

/// Message to trigger config save
#[derive(Message)]
pub struct SaveConfigRequest;

/// Message to set the default library path
#[derive(Message)]
pub struct SetDefaultLibraryRequest {
    pub path: PathBuf,
}

/// Message to add a library to the recent list
#[derive(Message)]
pub struct AddRecentLibraryRequest {
    pub path: PathBuf,
}

/// Message to update the last map path in config
#[derive(Message)]
pub struct UpdateLastMapPathRequest {
    pub path: PathBuf,
}

/// Get the path to the config file (platform-appropriate location)
fn get_config_path() -> PathBuf {
    crate::paths::config_file()
}

/// Result of loading config from disk
struct LoadConfigResult {
    config: AppConfig,
    /// Error message if config was reset to defaults due to an error
    reset_reason: Option<String>,
}

/// Load configuration from disk
fn load_config() -> LoadConfigResult {
    let config_path = get_config_path();

    let (data, reset_reason) = if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(json) => match serde_json::from_str(&json) {
                Ok(data) => {
                    info!("Loaded config from {:?}", config_path);
                    (data, None)
                }
                Err(e) => {
                    warn!("Failed to parse config file: {}", e);
                    (
                        AppConfigData::default(),
                        Some(format!("Configuration file was corrupted: {}", e)),
                    )
                }
            },
            Err(e) => {
                warn!("Failed to read config file: {}", e);
                (
                    AppConfigData::default(),
                    Some(format!("Could not read configuration file: {}", e)),
                )
            }
        }
    } else {
        info!("No config file found, using defaults");
        (AppConfigData::default(), None)
    };

    LoadConfigResult {
        config: AppConfig {
            data,
            config_path,
            dirty: false,
        },
        reset_reason,
    }
}

/// Save configuration to disk
fn save_config(config: &AppConfig) {
    match serde_json::to_string_pretty(&config.data) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&config.config_path, json) {
                error!("Failed to save config: {}", e);
            } else {
                info!("Config saved to {:?}", config.config_path);
            }
        }
        Err(e) => {
            error!("Failed to serialize config: {}", e);
        }
    }
}

/// Startup system to load config from disk into the existing resource
fn load_config_system(
    mut config: ResMut<AppConfig>,
    mut reset_notification: ResMut<ConfigResetNotification>,
) {
    let result = load_config();
    config.data = result.config.data;
    config.config_path = result.config.config_path;
    config.dirty = result.config.dirty;

    // Set notification if config was reset due to an error
    if let Some(reason) = result.reset_reason {
        reset_notification.show = true;
        reset_notification.reason = Some(reason);
    }
}

/// Startup system to check if last map exists
fn check_last_map_exists(config: Res<AppConfig>, mut warning: ResMut<MissingMapWarning>) {
    if let Some(ref path) = config.data.last_map_path
        && !path.exists()
    {
        warning.show = true;
        warning.path = Some(path.clone());
        info!("Last opened map no longer exists: {:?}", path);
    }
}

/// System to save config when requested
fn save_config_system(
    mut events: MessageReader<SaveConfigRequest>,
    mut config: ResMut<AppConfig>,
) {
    for _ in events.read() {
        if config.dirty {
            save_config(&config);
            config.dirty = false;
        }
    }
}

/// System to set the default library path
fn set_default_library_system(
    mut events: MessageReader<SetDefaultLibraryRequest>,
    mut config: ResMut<AppConfig>,
    mut save_events: MessageWriter<SaveConfigRequest>,
) {
    for event in events.read() {
        config.data.default_library_path = Some(event.path.clone());
        config.dirty = true;
        save_events.write(SaveConfigRequest);
        info!("Set default library to {:?}", event.path);
    }
}

/// System to add a library to the recent list
fn add_recent_library_system(
    mut events: MessageReader<AddRecentLibraryRequest>,
    mut config: ResMut<AppConfig>,
    mut save_events: MessageWriter<SaveConfigRequest>,
) {
    for event in events.read() {
        // Remove if already in list (to move it to front)
        config
            .data
            .recent_libraries
            .retain(|p| p != &event.path);

        // Add to front
        config.data.recent_libraries.insert(0, event.path.clone());

        // Trim to max size
        config.data.recent_libraries.truncate(MAX_RECENT_LIBRARIES);

        config.dirty = true;
        save_events.write(SaveConfigRequest);
    }
}

/// System to update last map path
fn update_last_map_path_system(
    mut events: MessageReader<UpdateLastMapPathRequest>,
    mut config: ResMut<AppConfig>,
    mut save_events: MessageWriter<SaveConfigRequest>,
) {
    for event in events.read() {
        config.data.last_map_path = Some(event.path.clone());
        config.dirty = true;
        save_events.write(SaveConfigRequest);
    }
}

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AppConfig>()
            .init_resource::<MissingMapWarning>()
            .init_resource::<ConfigResetNotification>()
            .add_message::<SaveConfigRequest>()
            .add_message::<SetDefaultLibraryRequest>()
            .add_message::<AddRecentLibraryRequest>()
            .add_message::<UpdateLastMapPathRequest>()
            .add_systems(
                Startup,
                (load_config_system, check_last_map_exists)
                    .chain()
                    .in_set(ConfigLoaded),
            )
            .add_systems(
                Update,
                (
                    save_config_system.run_if(on_message::<SaveConfigRequest>),
                    set_default_library_system.run_if(on_message::<SetDefaultLibraryRequest>),
                    add_recent_library_system.run_if(on_message::<AddRecentLibraryRequest>),
                    update_last_map_path_system.run_if(on_message::<UpdateLastMapPathRequest>),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_data_default() {
        let data = AppConfigData::default();
        assert!(data.default_library_path.is_none());
        assert!(data.recent_libraries.is_empty());
        assert!(data.last_map_path.is_none());
    }

    #[test]
    fn test_app_config_data_serialization() {
        let data = AppConfigData {
            default_library_path: Some(PathBuf::from("/path/to/library")),
            recent_libraries: vec![
                PathBuf::from("/path/one"),
                PathBuf::from("/path/two"),
            ],
            last_map_path: Some(PathBuf::from("/path/to/map.json")),
        };

        let json = serde_json::to_string(&data).unwrap();
        let parsed: AppConfigData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.default_library_path, data.default_library_path);
        assert_eq!(parsed.recent_libraries, data.recent_libraries);
        assert_eq!(parsed.last_map_path, data.last_map_path);
    }

    #[test]
    fn test_missing_map_warning_default() {
        let warning = MissingMapWarning::default();
        assert!(!warning.show);
        assert!(warning.path.is_none());
    }
}
