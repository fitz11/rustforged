//! Result types for async map operations.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::resources::SaveConflict;
use crate::assets::{AssetFingerprint, FingerprintCache};
use crate::map::SavedMap;

/// Result of an async save operation
pub struct SaveResult {
    pub path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
    /// Set when the save stopped because the file on disk changed
    pub conflict: Option<SaveConflict>,
    /// Content hash of the file as written
    pub disk_hash: Option<String>,
    /// Id written to the file
    pub map_id: String,
    /// Fingerprints written to the file
    pub asset_fingerprints: BTreeMap<String, AssetFingerprint>,
    /// Fingerprints computed while saving, for the library's cache
    pub computed: FingerprintCache,
}

/// Result of an async load operation
pub struct LoadResult {
    pub path: PathBuf,
    /// The map, with placed items already pointed at their resolved asset paths
    pub saved_map: Option<SavedMap>,
    pub error: Option<String>,
    /// Content hash of the file as read
    pub disk_hash: Option<String>,
    /// References found under a different path: (old path, new path)
    pub relinked: Vec<(String, String)>,
    /// References with no matching asset in the library, each with the one library file
    /// that has the same name but different contents, if there is one
    pub missing: Vec<(String, Option<String>)>,
    /// Fingerprints computed while resolving, for the library's cache
    pub computed: FingerprintCache,
}
