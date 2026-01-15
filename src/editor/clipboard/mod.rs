//! Clipboard system for copy/cut/paste operations.
//!
//! This module provides clipboard functionality for copying, cutting, and pasting
//! map items and annotations. Items are stored with their offset from the selection
//! centroid, allowing them to be pasted while maintaining relative positions.
//!
//! ## Module Structure
//!
//! - [`types`] - Clipboard data types (ClipboardPlacedItem, ClipboardPath, etc.)
//! - [`helpers`] - Color conversion and centroid calculation utilities
//! - [`copy`] - Copy system (Ctrl+C)
//! - [`cut`] - Cut system (Ctrl+X)
//! - [`paste`] - Paste system (Ctrl+V)
//!
//! ## Key Types
//!
//! - [`Clipboard`]: Resource holding copied items with their offsets
//! - [`ClipboardPlacedItem`]: Placed item data for clipboard
//! - [`ClipboardPath`]: Path annotation data for clipboard
//! - [`ClipboardLine`]: Line annotation data for clipboard
//! - [`ClipboardText`]: Text annotation data for clipboard
//!
//! ## Systems
//!
//! - [`handle_copy`]: Copy selected items to clipboard (Ctrl+C)
//! - [`handle_cut`]: Cut selected items to clipboard (Ctrl+X)
//! - [`handle_paste`]: Paste clipboard items at cursor position (Ctrl+V)

mod copy;
mod cut;
mod helpers;
mod paste;
mod tests;
mod types;

// Re-exports - Types
pub use types::Clipboard;

// These types are used in tests and for completeness
#[allow(unused_imports)]
pub use types::{ClipboardLine, ClipboardPath, ClipboardPlacedItem, ClipboardText};

// Re-exports - Systems
pub use copy::handle_copy;
pub use cut::handle_cut;
pub use paste::handle_paste;
