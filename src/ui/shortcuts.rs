//! Every keyboard and mouse shortcut, in one table. The Help window shows it, and a test
//! checks the README lists every one, so the two can't drift apart from the code again.
//!
//! "Ctrl" is Cmd on macOS; [`display_keys`] shows the right one.

/// One shortcut: the keys (or mouse action) and what it does.
pub struct Shortcut {
    pub keys: &'static str,
    pub action: &'static str,
}

/// Shortcuts under one heading.
pub struct ShortcutGroup {
    pub title: &'static str,
    pub shortcuts: &'static [Shortcut],
}

const fn s(keys: &'static str, action: &'static str) -> Shortcut {
    Shortcut { keys, action }
}

pub const SHORTCUTS: &[ShortcutGroup] = &[
    ShortcutGroup {
        title: "Tools",
        shortcuts: &[
            s("V / S", "Select: click to select, drag to move"),
            s(
                "P",
                "Place: click to place the selected asset, then back to Select",
            ),
            s(
                "B",
                "Brush: drag to paint the selected asset into each cell",
            ),
            s("D", "Draw: freehand annotation paths"),
            s("L", "Line: straight line annotations"),
            s("T", "Text: click to type a note, click a note to edit it"),
            s(
                "F",
                "Fog: drag to reveal fog of war, right-drag to cover it",
            ),
            s("C / Shift+C", "Cycle the target layer (Place and Brush)"),
            s("H", "Show or hide this help"),
        ],
    },
    ShortcutGroup {
        title: "Placing",
        shortcuts: &[
            s("Shift+Click", "Place and keep the Place tool for another"),
            s("Ctrl+Click", "Place without snapping to the grid"),
            s("Shift+Drag (Brush)", "Fit each painted asset to one cell"),
        ],
    },
    ShortcutGroup {
        title: "Selection",
        shortcuts: &[
            s("Click", "Select the topmost item"),
            s("Ctrl+Click", "Add to or remove from the selection"),
            s("Drag on empty space", "Box select"),
            s("Drag", "Move the selection (Shift: whole cells)"),
            s(
                "Drag a handle",
                "Resize (Shift keeps the shape) or rotate (Shift: 15° steps)",
            ),
            s("Escape", "Clear the selection"),
            s("Delete / Backspace", "Delete the selection"),
            s("G", "Fit to one grid cell"),
            s("Shift+G", "Center in its grid cell"),
            s("A", "Restore the aspect ratio"),
            s("R / Shift+R", "Rotate 90° clockwise / counter-clockwise"),
        ],
    },
    ShortcutGroup {
        title: "Editing",
        shortcuts: &[
            s("Ctrl+C / Ctrl+X / Ctrl+V", "Copy / cut / paste"),
            s("Ctrl+Z", "Undo"),
            s("Ctrl+Y / Ctrl+Shift+Z", "Redo"),
        ],
    },
    ShortcutGroup {
        title: "Fog",
        shortcuts: &[
            s("Drag (Fog)", "Reveal with the brush"),
            s("Right-drag (Fog)", "Cover with the brush"),
            s(
                "Shift+Drag (Fog)",
                "Reveal (or with the right button, cover) a rectangle of cells",
            ),
        ],
    },
    ShortcutGroup {
        title: "Notes",
        shortcuts: &[
            s("Enter (Text)", "Keep the note"),
            s("Escape (Text)", "Cancel the edit"),
        ],
    },
    ShortcutGroup {
        title: "Camera",
        shortcuts: &[
            s("Middle-mouse drag / Space+Drag", "Pan"),
            s("Arrow keys", "Pan (faster with Shift)"),
            s("Scroll wheel", "Zoom toward the cursor"),
            s(
                "Trackpad scroll / pinch",
                "Pan / zoom (Ctrl+scroll zooms too)",
            ),
            s("Home", "Frame the map, or the player view during a session"),
        ],
    },
    ShortcutGroup {
        title: "Files",
        shortcuts: &[
            s("Ctrl+N", "New map"),
            s("Ctrl+O", "Open a map file"),
            s("Ctrl+S", "Save"),
            s("Ctrl+Shift+S", "Save as"),
            s("F2", "Rename the selected asset"),
            s("F3", "Rename the map"),
            s("F4", "Rename the library"),
        ],
    },
    ShortcutGroup {
        title: "Live session",
        shortcuts: &[
            s(
                "Ctrl+B",
                "Black out the player screen, or show the map again",
            ),
            s("Escape (player window)", "End the session"),
        ],
    },
];

/// `keys` as this platform names them: Cmd instead of Ctrl on macOS.
pub fn display_keys(keys: &str) -> String {
    if cfg!(target_os = "macos") {
        keys.replace("Ctrl", "Cmd")
    } else {
        keys.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readme_lists_every_shortcut() {
        let readme = include_str!("../../README.md");
        for group in SHORTCUTS {
            for shortcut in group.shortcuts {
                let row = format!("| {} | {} |", shortcut.keys, shortcut.action);
                assert!(readme.contains(&row), "README.md is missing the row: {row}");
            }
        }
    }
}
