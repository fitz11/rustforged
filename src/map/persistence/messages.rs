//! Message types for map persistence operations.

use bevy::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Message, Clone)]
pub struct SaveMapRequest {
    pub path: PathBuf,
    /// Write even if the file changed on disk since it was opened (the user chose to
    /// overwrite after a save conflict)
    pub overwrite: bool,
    /// Identifies this request in the [`MapSaveFinished`] sent when it's done
    pub ticket: u64,
}

impl SaveMapRequest {
    /// A save that stops and asks if the file changed on disk.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            overwrite: false,
            ticket: next_ticket(),
        }
    }

    /// A save that writes even though the file changed on disk.
    pub fn overwrite(path: PathBuf) -> Self {
        Self {
            overwrite: true,
            ..Self::new(path)
        }
    }
}

fn next_ticket() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Sent when a [`SaveMapRequest`] is done, whether or not the map was written.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapSaveFinished {
    /// The request's [`SaveMapRequest::ticket`]
    pub ticket: u64,
    pub outcome: SaveOutcome,
}

/// How a save ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOutcome {
    /// The map was written to disk
    Saved,
    /// The file changed on disk; the save conflict dialog is asking what to do
    Conflict,
    /// Nothing was written: the save failed, or another save or load was in progress
    Failed,
}

#[derive(Message, Clone)]
pub struct LoadMapRequest {
    pub path: PathBuf,
    /// Keep the map being edited open in the background (another scene) instead of
    /// replacing it
    pub alongside: bool,
}

impl LoadMapRequest {
    /// Open `path` in place of the map being edited.
    pub fn replace(path: PathBuf) -> Self {
        Self {
            path,
            alongside: false,
        }
    }

    /// Open `path` too, keeping the map being edited open in the background.
    pub fn alongside(path: PathBuf) -> Self {
        Self {
            path,
            alongside: true,
        }
    }
}

#[derive(Message)]
pub struct NewMapRequest;

/// Sent when the map being edited was replaced by another: a new map, a map opened from
/// disk, or a switch to another open map. Editor state about the old map's entities (undo
/// history, a brush stroke in progress) no longer applies.
#[derive(Message)]
pub struct MapReplaced;

/// Message to request switching to a different open map
#[derive(Message)]
pub struct SwitchMapRequest {
    pub map_id: u64,
    /// Close the map being switched away from instead of keeping it open
    pub close_current: bool,
}
