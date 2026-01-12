use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use bevy_egui::EguiContexts;

use crate::map::{Layer, Selected};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTool {
    #[default]
    Select,
    Place,
    Erase,
    Draw,
    Line,
    Text,
}

impl EditorTool {
    pub fn display_name(&self) -> &'static str {
        match self {
            EditorTool::Select => "Select (V)",
            EditorTool::Place => "Place (B)",
            EditorTool::Erase => "Erase (X)",
            EditorTool::Draw => "Draw (D)",
            EditorTool::Line => "Line (L)",
            EditorTool::Text => "Text (T)",
        }
    }

    pub fn cursor_icon(&self) -> CursorIcon {
        match self {
            EditorTool::Select => CursorIcon::System(SystemCursorIcon::Default),
            EditorTool::Place => CursorIcon::System(SystemCursorIcon::Crosshair),
            EditorTool::Erase => CursorIcon::System(SystemCursorIcon::NotAllowed),
            EditorTool::Draw => CursorIcon::System(SystemCursorIcon::Crosshair),
            EditorTool::Line => CursorIcon::System(SystemCursorIcon::Crosshair),
            EditorTool::Text => CursorIcon::System(SystemCursorIcon::Text),
        }
    }

    pub fn all() -> &'static [EditorTool] {
        &[
            EditorTool::Select,
            EditorTool::Place,
            EditorTool::Erase,
            EditorTool::Draw,
            EditorTool::Line,
            EditorTool::Text,
        ]
    }

    pub fn is_annotation_tool(&self) -> bool {
        matches!(self, EditorTool::Draw | EditorTool::Line | EditorTool::Text)
    }
}

#[derive(Resource, Default)]
pub struct CurrentTool {
    pub tool: EditorTool,
}

#[derive(Resource)]
pub struct SelectedLayer {
    pub layer: Layer,
}

impl Default for SelectedLayer {
    fn default() -> Self {
        Self {
            layer: Layer::Token,
        }
    }
}

pub fn handle_tool_shortcuts(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut current_tool: ResMut<CurrentTool>,
    selected_query: Query<Entity, With<Selected>>,
    mut contexts: EguiContexts,
) {
    // Don't change tools if typing in a text field
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.wants_keyboard_input()
    {
        return;
    }

    let new_tool = if keyboard.just_pressed(KeyCode::KeyV) || keyboard.just_pressed(KeyCode::KeyS) {
        Some(EditorTool::Select)
    } else if keyboard.just_pressed(KeyCode::KeyB) || keyboard.just_pressed(KeyCode::KeyP) {
        Some(EditorTool::Place)
    } else if keyboard.just_pressed(KeyCode::KeyX) || keyboard.just_pressed(KeyCode::KeyE) {
        Some(EditorTool::Erase)
    } else if keyboard.just_pressed(KeyCode::KeyD) {
        Some(EditorTool::Draw)
    } else if keyboard.just_pressed(KeyCode::KeyL) {
        Some(EditorTool::Line)
    } else if keyboard.just_pressed(KeyCode::KeyT) {
        Some(EditorTool::Text)
    } else {
        None
    };

    if let Some(tool) = new_tool {
        // Clear selection when switching tools
        if tool != current_tool.tool {
            for entity in selected_query.iter() {
                commands.entity(entity).remove::<Selected>();
            }
        }
        current_tool.tool = tool;
    }
}

pub fn update_cursor_icon(
    current_tool: Res<CurrentTool>,
    mut window_query: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut commands: Commands,
    mut contexts: EguiContexts,
) {
    let Ok((entity, _window)) = window_query.single_mut() else {
        return;
    };

    // Use default cursor over UI, tool cursor in editor space
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.is_pointer_over_area()
    {
        commands
            .entity(entity)
            .insert(CursorIcon::System(SystemCursorIcon::Default));
        return;
    }

    commands.entity(entity).insert(current_tool.tool.cursor_icon());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_names() {
        assert_eq!(EditorTool::Select.display_name(), "Select (V)");
        assert_eq!(EditorTool::Place.display_name(), "Place (B)");
        assert_eq!(EditorTool::Erase.display_name(), "Erase (X)");
        assert_eq!(EditorTool::Draw.display_name(), "Draw (D)");
        assert_eq!(EditorTool::Line.display_name(), "Line (L)");
        assert_eq!(EditorTool::Text.display_name(), "Text (T)");
    }

    #[test]
    fn test_display_names_contain_shortcuts() {
        // Each display name should contain its keyboard shortcut in parentheses
        for tool in EditorTool::all() {
            let name = tool.display_name();
            assert!(name.contains('('), "Display name should contain shortcut: {}", name);
            assert!(name.contains(')'), "Display name should contain shortcut: {}", name);
        }
    }

    #[test]
    fn test_all_returns_all_tools() {
        let all = EditorTool::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&EditorTool::Select));
        assert!(all.contains(&EditorTool::Place));
        assert!(all.contains(&EditorTool::Erase));
        assert!(all.contains(&EditorTool::Draw));
        assert!(all.contains(&EditorTool::Line));
        assert!(all.contains(&EditorTool::Text));
    }

    #[test]
    fn test_is_annotation_tool() {
        // Non-annotation tools
        assert!(!EditorTool::Select.is_annotation_tool());
        assert!(!EditorTool::Place.is_annotation_tool());
        assert!(!EditorTool::Erase.is_annotation_tool());

        // Annotation tools
        assert!(EditorTool::Draw.is_annotation_tool());
        assert!(EditorTool::Line.is_annotation_tool());
        assert!(EditorTool::Text.is_annotation_tool());
    }

    #[test]
    fn test_default_tool_is_select() {
        assert_eq!(EditorTool::default(), EditorTool::Select);
    }

    #[test]
    fn test_current_tool_default() {
        let current = CurrentTool::default();
        assert_eq!(current.tool, EditorTool::Select);
    }

    #[test]
    fn test_selected_layer_default() {
        let selected = SelectedLayer::default();
        assert_eq!(selected.layer, Layer::Token);
    }

    #[test]
    fn test_cursor_icons_are_system_cursors() {
        // All tools should return system cursor icons
        for tool in EditorTool::all() {
            let icon = tool.cursor_icon();
            assert!(matches!(icon, CursorIcon::System(_)));
        }
    }

    #[test]
    fn test_drawing_tools_have_crosshair() {
        // Place, Draw, and Line tools should use crosshair
        assert_eq!(
            EditorTool::Place.cursor_icon(),
            CursorIcon::System(SystemCursorIcon::Crosshair)
        );
        assert_eq!(
            EditorTool::Draw.cursor_icon(),
            CursorIcon::System(SystemCursorIcon::Crosshair)
        );
        assert_eq!(
            EditorTool::Line.cursor_icon(),
            CursorIcon::System(SystemCursorIcon::Crosshair)
        );
    }

    #[test]
    fn test_text_tool_has_text_cursor() {
        assert_eq!(
            EditorTool::Text.cursor_icon(),
            CursorIcon::System(SystemCursorIcon::Text)
        );
    }
}
