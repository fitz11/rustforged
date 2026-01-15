//! Result types for async map operations.

use std::path::PathBuf;

use crate::map::SavedMap;

/// Result of an async save operation
pub struct SaveResult {
    pub path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

/// Result of an async load operation
pub struct LoadResult {
    pub path: PathBuf,
    pub saved_map: Option<SavedMap>,
    pub error: Option<String>,
}
