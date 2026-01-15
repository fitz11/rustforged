//! Unit tests for the update module.

use super::manifest::{current_platform_key, get_platform_asset, is_newer_version, ReleaseManifest};
use super::state::UpdateCheckResult;
use std::collections::HashMap;

// Version comparison tests
#[test]
fn test_is_newer_version_true() {
    assert!(is_newer_version("1.1.0", "1.0.0"));
    assert!(is_newer_version("2.0.0", "1.9.9"));
    assert!(is_newer_version("1.0.1", "1.0.0"));
}

#[test]
fn test_is_newer_version_false() {
    assert!(!is_newer_version("1.0.0", "1.0.0")); // Same version
    assert!(!is_newer_version("1.0.0", "1.1.0")); // Older
    assert!(!is_newer_version("0.9.0", "1.0.0")); // Much older
}

#[test]
fn test_is_newer_version_invalid() {
    assert!(!is_newer_version("invalid", "1.0.0"));
    assert!(!is_newer_version("1.0.0", "invalid"));
    assert!(!is_newer_version("", "1.0.0"));
}

// Manifest parsing tests
#[test]
fn test_manifest_parsing_full() {
    let json = r#"{
        "version": "1.2.3",
        "release_url": "https://example.com/releases/v1.2.3",
        "release_notes": "New features",
        "assets": {
            "windows-x64": "https://example.com/win.msi",
            "macos-aarch64": "https://example.com/mac.dmg"
        }
    }"#;

    let manifest: ReleaseManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.version, "1.2.3");
    assert_eq!(manifest.release_url, "https://example.com/releases/v1.2.3");
    assert_eq!(manifest.release_notes, Some("New features".to_string()));
    assert_eq!(manifest.assets.len(), 2);
}

#[test]
fn test_manifest_parsing_minimal() {
    let json = r#"{
        "version": "1.0.0",
        "release_url": "https://example.com"
    }"#;

    let manifest: ReleaseManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.version, "1.0.0");
    assert!(manifest.release_notes.is_none());
    assert!(manifest.assets.is_empty());
}

// Platform asset selection tests
#[test]
fn test_get_platform_asset_found() {
    let mut assets = HashMap::new();
    assets.insert(
        current_platform_key().to_string(),
        "https://example.com/installer".to_string(),
    );

    let result = get_platform_asset(&assets);
    assert_eq!(result, Some("https://example.com/installer".to_string()));
}

#[test]
fn test_get_platform_asset_missing() {
    let assets = HashMap::new();
    let result = get_platform_asset(&assets);
    assert!(result.is_none());
}

#[test]
fn test_get_platform_asset_wrong_platform() {
    let mut assets = HashMap::new();
    // Insert a platform that doesn't match current
    assets.insert(
        "nonexistent-platform".to_string(),
        "https://example.com/installer".to_string(),
    );

    let result = get_platform_asset(&assets);
    assert!(result.is_none());
}

// UpdateCheckResult helper tests
#[test]
fn test_update_check_result_no_update() {
    let result = UpdateCheckResult::no_update();
    assert!(!result.update_available);
    assert!(result.latest_version.is_none());
    assert!(result.release_url.is_none());
    assert!(result.error.is_none());
}

#[test]
fn test_update_check_result_error() {
    let result = UpdateCheckResult::error("Network error".to_string());
    assert!(!result.update_available);
    assert_eq!(result.error, Some("Network error".to_string()));
}

// Platform key tests
#[test]
fn test_current_platform_key_not_empty() {
    let key = current_platform_key();
    assert!(!key.is_empty());
    // Should be one of the known platforms or "unknown"
    let valid_keys = [
        "windows-x64",
        "windows-arm64",
        "macos-aarch64",
        "linux-x64",
        "unknown",
    ];
    assert!(valid_keys.contains(&key));
}
