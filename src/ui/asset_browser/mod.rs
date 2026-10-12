//! Asset browser panel module.
//!
//! This module provides the left-side asset browser panel UI for managing
//! asset libraries, maps, and browsing/selecting assets for placement.
//!
//! ## Module Structure
//!
//! - [`state`] - AssetBrowserState resource and SystemParam bundles
//! - [`helpers`] - Helper functions (folder discovery, map scanning, colors)
//! - [`library_ops`] - Library export/import operations (zip handling)
//! - [`asset_ops`] - Asset file operations (rename, move)
//! - [`thumbnails`] - Thumbnail loading and registration system
//! - [`dialogs`] - Dialog windows (rename, move, import errors)
//! - [`main_panel`] - Main asset browser UI system, laying out the sections below
//! - [`library_section`] - Open, create, refresh, export and import libraries
//! - [`maps_section`] - Open maps, New/Open/Save, and the library's map list
//! - [`assets_section`] - Folders, the asset list and the selected asset
//!
//! ## Key Types
//!
//! - [`AssetBrowserState`]: Resource tracking browser panel state
//! - [`MapResources`]: SystemParam bundling map-related resources
//! - [`DialogStates`]: SystemParam bundling dialog state resources
//!
//! ## Systems
//!
//! - [`asset_browser_ui`]: Main asset browser panel rendering system
//! - [`load_and_register_thumbnails`]: Thumbnail loading system

mod asset_ops;
mod assets_section;
mod dialogs;
mod helpers;
mod library_ops;
mod library_section;
mod main_panel;
mod maps_section;
mod state;
mod thumbnails;

// Re-exports - State
pub use state::AssetBrowserState;

// These are used internally but exposed for plugin registration patterns
#[allow(unused_imports)]
pub use state::{DialogStates, MapResources};

// Re-exports - Systems
pub use main_panel::asset_browser_ui;
pub use thumbnails::load_and_register_thumbnails;
