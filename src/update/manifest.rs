//! Release manifest handling and version comparison.

use semver::Version;
use serde::Deserialize;
use std::collections::HashMap;

/// URL to the release manifest file.
/// Change this constant to migrate away from GitHub hosting.
pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/fitz11/rustforged/main/releases/latest.json";

/// Release manifest structure - can be hosted anywhere.
///
/// Example JSON:
/// ```json
/// {
///   "version": "1.2.3",
///   "release_url": "https://github.com/fitz11/rustforged/releases/v1.2.3",
///   "release_notes": "Bug fixes and improvements...",
///   "assets": {
///     "windows-x64": "https://example.com/rustforged-1.2.3-x64.msi",
///     "windows-arm64": "https://example.com/rustforged-1.2.3-arm64.msi",
///     "macos-aarch64": "https://example.com/rustforged-1.2.3-aarch64.dmg"
///   }
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct ReleaseManifest {
    /// Latest version string (semver format, e.g., "1.2.3")
    pub version: String,
    /// URL to the release page for manual download
    pub release_url: String,
    /// Release notes/changelog (optional)
    pub release_notes: Option<String>,
    /// Platform-specific download URLs (key: platform, value: URL)
    #[serde(default)]
    pub assets: HashMap<String, String>,
}

/// Compare versions, returns true if `latest` is newer than `current`
pub fn is_newer_version(latest: &str, current: &str) -> bool {
    match (Version::parse(latest), Version::parse(current)) {
        (Ok(latest_v), Ok(current_v)) => latest_v > current_v,
        _ => false,
    }
}

/// Get the platform key for the current build target
pub fn current_platform_key() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-x64"
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        "windows-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-aarch64"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x64"
    }
    // Fallback for other configurations (e.g., macOS x86_64)
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64")
    )))]
    {
        "unknown"
    }
}

/// Get download URL for current platform from assets map
pub fn get_platform_asset(assets: &HashMap<String, String>) -> Option<String> {
    assets.get(current_platform_key()).cloned()
}
