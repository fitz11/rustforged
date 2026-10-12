//! Keyboard modifier helpers shared by the editor and UI shortcuts.
//!
//! The "command" modifier is Ctrl, plus Cmd on macOS, so shortcuts like Ctrl+S work the
//! way Mac users expect.

use bevy::prelude::*;

/// Ctrl, or Cmd on macOS, is held.
pub fn command_held(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
        || (cfg!(target_os = "macos")
            && keyboard.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]))
}

/// Either Shift key is held.
pub fn shift_held(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
}

/// Either Alt (Option on macOS) key is held.
pub fn alt_held(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight])
}

/// `key` was just pressed with no Ctrl, Cmd or Alt held, so Ctrl+S doesn't also count as
/// the bare S shortcut. Shift is allowed (it varies some shortcuts, like Shift+C).
pub fn plain_key_just_pressed(keyboard: &ButtonInput<KeyCode>, key: KeyCode) -> bool {
    keyboard.just_pressed(key)
        && !command_held(keyboard)
        && !alt_held(keyboard)
        && !keyboard.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight])
}

/// `key` was just pressed together with Ctrl (Cmd on macOS).
pub fn command_key_just_pressed(keyboard: &ButtonInput<KeyCode>, key: KeyCode) -> bool {
    keyboard.just_pressed(key) && command_held(keyboard)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(held: &[KeyCode], pressed: KeyCode) -> ButtonInput<KeyCode> {
        let mut input = ButtonInput::default();
        for key in held {
            input.press(*key);
        }
        input.clear();
        input.press(pressed);
        input
    }

    #[test]
    fn test_plain_key_ignores_ctrl_combinations() {
        assert!(plain_key_just_pressed(
            &keys(&[], KeyCode::KeyS),
            KeyCode::KeyS
        ));
        assert!(!plain_key_just_pressed(
            &keys(&[KeyCode::ControlLeft], KeyCode::KeyS),
            KeyCode::KeyS
        ));
        assert!(!plain_key_just_pressed(
            &keys(&[KeyCode::AltRight], KeyCode::KeyS),
            KeyCode::KeyS
        ));
    }

    #[test]
    fn test_plain_key_allows_shift() {
        assert!(plain_key_just_pressed(
            &keys(&[KeyCode::ShiftLeft], KeyCode::KeyC),
            KeyCode::KeyC
        ));
    }

    #[test]
    fn test_command_key_needs_ctrl() {
        assert!(command_key_just_pressed(
            &keys(&[KeyCode::ControlRight], KeyCode::KeyS),
            KeyCode::KeyS
        ));
        assert!(!command_key_just_pressed(
            &keys(&[], KeyCode::KeyS),
            KeyCode::KeyS
        ));
    }
}
