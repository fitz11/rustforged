//! Run conditions for controlling when editor systems execute.
//!
//! These conditions help optimize performance by preventing systems from running
//! when they have no work to do.

use bevy::prelude::*;

use crate::editor::tools::{CurrentTool, EditorTool};
use crate::session::state::LiveSessionState;
use crate::ui::DialogState;

/// Run condition: returns true when the current tool matches the specified tool.
///
/// Usage: `.run_if(tool_is(EditorTool::Place))`
pub fn tool_is(tool: EditorTool) -> impl FnMut(Res<CurrentTool>) -> bool + Clone {
    move |current: Res<CurrentTool>| current.tool == tool
}

/// Run condition: returns true when the live session is active.
///
/// Usage: `.run_if(session_is_active)`
pub fn session_is_active(state: Res<LiveSessionState>) -> bool {
    state.is_active
}

/// Run condition: returns true when no modal dialog is open.
///
/// Use this to prevent editor input handlers from processing when
/// the user is interacting with a dialog.
///
/// Usage: `.run_if(no_dialog_open)`
pub fn no_dialog_open(dialog_state: Res<DialogState>) -> bool {
    !dialog_state.any_modal_open
}
