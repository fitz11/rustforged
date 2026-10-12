//! Run conditions for controlling when editor systems execute.
//!
//! These conditions help optimize performance by preventing systems from running
//! when they have no work to do.

use bevy::prelude::*;

use crate::input::{command_key_just_pressed, plain_key_just_pressed};

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

/// Run condition: keyboard shortcuts may run, because no modal dialog is open and no text
/// field has keyboard focus.
///
/// Usage: `.run_if(shortcuts_enabled)`
pub fn shortcuts_enabled(dialog_state: Res<DialogState>) -> bool {
    !dialog_state.any_modal_open && !dialog_state.text_input_focused
}

/// Run condition: no text field has keyboard focus. For shortcuts that also work while a
/// window is open, like H closing Help.
pub fn no_text_input(dialog_state: Res<DialogState>) -> bool {
    !dialog_state.text_input_focused
}

/// Run condition: `key` was just pressed without Ctrl, Cmd or Alt.
///
/// Usage: `.run_if(plain_key(KeyCode::KeyG))`
pub fn plain_key(key: KeyCode) -> impl FnMut(Res<ButtonInput<KeyCode>>) -> bool + Clone {
    move |keyboard: Res<ButtonInput<KeyCode>>| plain_key_just_pressed(&keyboard, key)
}

/// Run condition: `key` was just pressed with Ctrl (Cmd on macOS).
///
/// Usage: `.run_if(command_key(KeyCode::KeyC))`
pub fn command_key(key: KeyCode) -> impl FnMut(Res<ButtonInput<KeyCode>>) -> bool + Clone {
    move |keyboard: Res<ButtonInput<KeyCode>>| command_key_just_pressed(&keyboard, key)
}
