//! Layers panel UI module.
//!
//! This module provides the right-side panel UI for managing layers,
//! fog of war, item properties, and live session controls.
//!
//! ## Module Structure
//!
//! - [`layers`] - Layer visibility and lock controls
//! - [`fog`] - Fog of War toggle and reset controls
//! - [`properties`] - Selected item properties editor
//! - [`session`] - Live Session viewport controls
//! - [`main_panel`] - Main panel orchestration
//! - [`help`] - Help popup and keyboard shortcut
//!
//! ## Key Types
//!
//! - [`HelpWindowState`]: Resource tracking help window visibility
//!
//! ## Systems
//!
//! - [`layers_panel_ui`]: Main layers panel rendering system
//! - [`help_popup_ui`]: Help popup window rendering system
//! - [`handle_help_shortcut`]: Keyboard shortcut handler for help window

use bevy::prelude::*;

mod fog;
mod help;
mod layers;
mod main_panel;
mod properties;
mod session;

/// Resource to track whether the help window is open.
#[derive(Resource, Default)]
pub struct HelpWindowState {
    pub is_open: bool,
}

// Re-exports - Systems
pub use help::{handle_help_shortcut, help_popup_ui};
pub use main_panel::layers_panel_ui;
