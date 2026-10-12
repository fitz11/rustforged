//! Core update operations: checking, downloading, and installing.

use super::CURRENT_VERSION;
use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::manifest::{
    MANIFEST_URL, ReleaseManifest, get_platform_asset, get_platform_checksum, is_newer_version,
};
use super::state::{DownloadResult, UpdateCheckResult};

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
                let download_sha256 = get_platform_checksum(&manifest.checksums);

                UpdateCheckResult {
                    update_available,
                    latest_version: Some(manifest.version),
                    release_url: Some(manifest.release_url),
                    release_notes: manifest.release_notes,
                    download_url,
                    download_sha256,
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

/// File extension of the installer at `url` (e.g. `.msi` or `-setup.exe` keeps `.exe`), so
/// the downloaded file is launched the right way.
pub(super) fn installer_extension(url: &str) -> &'static str {
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    [".msi", ".exe", ".dmg", ".tar.gz"]
        .into_iter()
        .find(|ext| name.ends_with(ext))
        .unwrap_or(".bin")
}

fn download_error(message: String) -> DownloadResult {
    DownloadResult {
        success: false,
        path: None,
        error: Some(message),
    }
}

/// Copies `reader` into `writer`, returning the SHA-256 of what was copied as lowercase hex.
pub(super) fn copy_and_hash(
    mut reader: impl Read,
    mut writer: impl Write,
) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        writer.write_all(&buffer[..read])?;
    }
    writer.flush()?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Download the installer to a temp file, keeping it only if it matches `expected_sha256`.
pub fn download_installer(
    url: String,
    version: String,
    expected_sha256: Option<String>,
) -> DownloadResult {
    if !url.starts_with("https://") {
        return download_error(format!(
            "Refusing to download over an insecure URL: {}",
            url
        ));
    }
    let Some(expected_sha256) = expected_sha256 else {
        return download_error(
            "This release doesn't publish a checksum for the installer, so it can't be \
             verified. Download it from the release page instead."
                .to_string(),
        );
    };

    let response = match ureq::get(&url)
        .set("User-Agent", "rustforged-updater")
        .call()
    {
        Ok(response) => response,
        Err(e) => return download_error(format!("Download failed: {}", e)),
    };

    // A fresh file of our own in the temp folder (deleted again if anything goes wrong)
    let mut file = match tempfile::Builder::new()
        .prefix(&format!("rustforged-{}-", version))
        .suffix(installer_extension(&url))
        .tempfile()
    {
        Ok(file) => file,
        Err(e) => return download_error(format!("Failed to create file: {}", e)),
    };

    let actual = match copy_and_hash(response.into_reader(), &mut file) {
        Ok(hash) => hash,
        Err(e) => return download_error(format!("Download failed: {}", e)),
    };
    if actual != expected_sha256 {
        return download_error(
            "The downloaded installer doesn't match the published checksum, so it wasn't \
             kept. Try again, or download it from the release page."
                .to_string(),
        );
    }

    match file.keep() {
        Ok((_, path)) => DownloadResult {
            success: true,
            path: Some(path),
            error: None,
        },
        Err(e) => download_error(format!("Failed to keep the download: {}", e.error)),
    }
}

/// Launch the downloaded installer. On Windows the app has to quit afterwards so the
/// installer can replace it; the caller does that once unsaved changes are dealt with.
#[allow(unused_variables)]
pub fn launch_installer(installer_path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let is_msi = installer_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("msi"));
        let result = if is_msi {
            // MSI installer in passive mode (progress bar, no questions)
            std::process::Command::new("msiexec")
                .arg("/i")
                .arg(installer_path)
                .arg("/passive")
                .spawn()
        } else {
            std::process::Command::new(installer_path).spawn()
        };
        result
            .map(|_| ())
            .map_err(|e| format!("Failed to launch installer: {}", e))
    }

    #[cfg(target_os = "macos")]
    {
        // Open DMG - user will drag to Applications
        std::process::Command::new("open")
            .arg(installer_path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open DMG: {}", e))
    }

    #[cfg(target_os = "linux")]
    {
        // Linux users should build from source
        Err("Auto-update not supported on Linux. Please build from source.".to_string())
    }
}
