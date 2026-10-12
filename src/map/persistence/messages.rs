//! Message types for map persistence operations.

use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Message)]
pub struct SaveMapRequest {
    pub path: PathBuf,
    /// Write even if the file changed on disk since it was opened (the user chose to
    /// overwrite after a save conflict)
    pub overwrite: bool,
}

impl SaveMapRequest {
    /// A save that stops and asks if the file changed on disk.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            overwrite: false,
        }
    }
}

#[derive(Message)]
pub struct LoadMapRequest {
    pub path: PathBuf,
}

#[derive(Message)]
pub struct NewMapRequest;

/// Message to request switching to a different open map
#[derive(Message)]
#[allow(dead_code)] // Reserved for future map switching feature
pub struct SwitchMapRequest {
    pub map_id: u64,
}
