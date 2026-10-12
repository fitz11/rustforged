//! Result types for async map operations.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::assets::{AssetFingerprint, FingerprintCache};
use crate::map::SavedMap;

/// Result of an async save operation
pub struct SaveResult {
    pub path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
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
    /// References found under a different path: (old path, new path)
    pub relinked: Vec<(String, String)>,
    /// References with no matching asset in the library
    pub missing: Vec<String>,
    /// Fingerprints computed while resolving, for the library's cache
    pub computed: FingerprintCache,
}
