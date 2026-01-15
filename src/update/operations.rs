//! Core update operations: checking, downloading, and installing.

use super::manifest::{get_platform_asset, is_newer_version, ReleaseManifest, MANIFEST_URL};
use super::state::{DownloadResult, UpdateCheckResult};
use super::CURRENT_VERSION;

/// Check for updates by fetching the release manifest
pub fn check_for_updates() -> UpdateCheckResult {
    let response = ureq::get(MANIFEST_URL)
        .set("User-Agent", "rustforged-update-checker")
        .call();

    match response {
        Ok(resp) => match resp.into_json::<ReleaseManifest>() {
            Ok(manifest) => {
                let update_available = is_newer_version(&manifest.version, CURRENT_VERSION);
                let download_url = if update_available {
                    get_platform_asset(&manifest.assets)
                } else {
                    None
                };

                UpdateCheckResult {
                    update_available,
                    latest_version: Some(manifest.version),
                    release_url: Some(manifest.release_url),
                    release_notes: manifest.release_notes,
                    download_url,
                    error: None,
                }
            }
            Err(e) => UpdateCheckResult::error(format!("Failed to parse manifest: {}", e)),
        },
        Err(ureq::Error::Status(404, _)) => {
            // No manifest yet - this is fine
            UpdateCheckResult::no_update()
        }
        Err(e) => UpdateCheckResult::error(format!("Failed to check for updates: {}", e)),
    }
}

/// Download the installer to a temp directory
pub fn download_installer(url: String, version: String) -> DownloadResult {
    let temp_dir = std::env::temp_dir();

    #[cfg(target_os = "windows")]
    let filename = format!("rustforged-{}.msi", version);

    #[cfg(target_os = "macos")]
    let filename = format!("rustforged-{}.dmg", version);

    #[cfg(target_os = "linux")]
    let filename = format!("rustforged-{}.tar.gz", version);

    let path = temp_dir.join(&filename);

    match ureq::get(&url)
        .set("User-Agent", "rustforged-updater")
        .call()
    {
        Ok(response) => {
            let mut file = match std::fs::File::create(&path) {
                Ok(f) => f,
                Err(e) => {
                    return DownloadResult {
                        success: false,
                        path: None,
                        error: Some(format!("Failed to create file: {}", e)),
                    }
                }
            };

            if let Err(e) = std::io::copy(&mut response.into_reader(), &mut file) {
                // Clean up partial file
                let _ = std::fs::remove_file(&path);
                return DownloadResult {
                    success: false,
                    path: None,
                    error: Some(format!("Download failed: {}", e)),
                };
            }

            DownloadResult {
                success: true,
                path: Some(path),
                error: None,
            }
        }
        Err(e) => DownloadResult {
            success: false,
            path: None,
            error: Some(format!("Download failed: {}", e)),
        },
    }
}

/// Launch the installer and exit the app
#[allow(unused_variables)]
pub fn install_and_restart(installer_path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Launch MSI installer in passive mode and exit
        std::process::Command::new("msiexec")
            .args(["/i", &installer_path.to_string_lossy(), "/passive"])
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;

        std::process::exit(0);
    }

    #[cfg(target_os = "macos")]
    {
        // Open DMG - user will drag to Applications
        std::process::Command::new("open")
            .arg(installer_path)
            .spawn()
            .map_err(|e| format!("Failed to open DMG: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        // Linux users should build from source
        Err("Auto-update not supported on Linux. Please build from source.".to_string())
    }
}
