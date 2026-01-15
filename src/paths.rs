//! Centralized path resolution for platform-appropriate user data directories.
//!
//! In development mode (cargo run), paths resolve to local directories.
//! In installed mode, paths resolve to platform-specific locations:
//! - Windows: `%APPDATA%\Rustforged\`
//! - macOS: `~/Library/Application Support/Rustforged/`
//! - Linux: `~/.config/rustforged/` (config), `~/.local/share/rustforged/` (data)

use std::path::{Path, PathBuf};

/// Returns true when running in development mode (cargo run).
///
/// Detection methods:
/// - `CARGO` env var is set (cargo run sets this)
/// - Debug assertions enabled (debug builds)
pub fn is_dev_mode() -> bool {
    std::env::var("CARGO").is_ok() || cfg!(debug_assertions)
}

/// Platform-appropriate config directory.
///
/// - Dev mode: current directory
/// - Linux: `~/.config/rustforged/`
/// - Windows/macOS: same as data_dir
pub fn config_dir() -> Option<PathBuf> {
    if is_dev_mode() {
        return Some(PathBuf::from("."));
    }

    #[cfg(target_os = "linux")]
    {
        dirs::config_dir().map(|p| p.join("rustforged"))
    }

    #[cfg(not(target_os = "linux"))]
    {
        data_dir()
    }
}

/// Platform-appropriate data directory.
///
/// - Dev mode: current directory
/// - Windows: `%APPDATA%\Rustforged\`
/// - macOS: `~/Library/Application Support/Rustforged/`
/// - Linux: `~/.local/share/rustforged/`
pub fn data_dir() -> Option<PathBuf> {
    if is_dev_mode() {
        return Some(PathBuf::from("."));
    }

    dirs::data_dir().map(|p| p.join("rustforged"))
}

/// Path to the config file.
///
/// - Dev mode: `./config.json`
/// - Installed: `{config_dir}/config.json`
pub fn config_file() -> PathBuf {
    config_dir()
        .map(|p| p.join("config.json"))
        .unwrap_or_else(|| PathBuf::from("config.json"))
}

/// Path to the default asset library.
///
/// - Dev mode: `./assets/library/`
/// - Installed: `{data_dir}/library/`
pub fn default_library_dir() -> PathBuf {
    data_dir()
        .map(|p| p.join("library"))
        .unwrap_or_else(|| PathBuf::from("assets/library"))
}

/// Path to the logs directory.
///
/// - Dev mode: `./logs/`
/// - Installed: `{data_dir}/logs/`
pub fn logs_dir() -> PathBuf {
    data_dir()
        .map(|p| p.join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"))
}

/// Path to bundled assets (fonts, etc.) that ship with the binary.
///
/// - Dev mode: `./assets/`
/// - Installed: `{exe_dir}/assets/`
#[allow(dead_code)]
pub fn bundled_assets_dir() -> PathBuf {
    if is_dev_mode() {
        return PathBuf::from("assets");
    }

    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("assets")))
        .unwrap_or_else(|| PathBuf::from("assets"))
}

/// Ensure all required directories exist.
///
/// Called early in startup to create config and data directories.
pub fn ensure_directories() -> std::io::Result<()> {
    if is_dev_mode() {
        // In dev mode, directories are local and typically exist
        return Ok(());
    }

    if let Some(config) = config_dir() {
        std::fs::create_dir_all(&config)?;
    }
    if let Some(data) = data_dir() {
        std::fs::create_dir_all(&data)?;
        std::fs::create_dir_all(data.join("logs"))?;
    }
    Ok(())
}

/// Create the default library structure on first run.
///
/// Creates empty terrain/, doodads/, tokens/, and maps/ subdirectories.
pub fn setup_default_library() -> std::io::Result<()> {
    let dest = default_library_dir();
    if dest.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(dest.join("terrain"))?;
    std::fs::create_dir_all(dest.join("doodads"))?;
    std::fs::create_dir_all(dest.join("tokens"))?;
    std::fs::create_dir_all(dest.join("maps"))?;
    Ok(())
}

/// Determines if a path is inside the Bevy assets folder.
/// Returns the path relative to assets/ if inside, None otherwise.
///
/// This is used to determine whether to use relative paths (for Bevy's asset loading)
/// or absolute paths (for external libraries).
pub fn get_bevy_assets_relative_path(path: &Path) -> Option<PathBuf> {
    let assets_dir = if is_dev_mode() {
        PathBuf::from("assets")
    } else {
        bundled_assets_dir()
    };

    let canonical_assets = assets_dir.canonicalize().ok()?;
    let canonical_path = path.canonicalize().ok()?;

    canonical_path
        .strip_prefix(&canonical_assets)
        .ok()
        .map(|p| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_not_none() {
        // In test mode (debug), should return Some
        assert!(config_dir().is_some());
    }

    #[test]
    fn test_data_dir_not_none() {
        assert!(data_dir().is_some());
    }

    #[test]
    fn test_config_file_has_json_extension() {
        let path = config_file();
        assert!(path.to_string_lossy().ends_with("config.json"));
    }

    #[test]
    fn test_dev_mode_returns_local_paths() {
        // In tests, is_dev_mode() should be true due to debug_assertions
        assert!(is_dev_mode());
        assert_eq!(config_dir(), Some(PathBuf::from(".")));
        assert_eq!(data_dir(), Some(PathBuf::from(".")));
    }
}
