use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use bevy_egui::EguiContexts;

use crate::map::Layer;

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
    keyboard: Res<ButtonInput<KeyCode>>,
    mut current_tool: ResMut<CurrentTool>,
    mut contexts: EguiContexts,
) {
    // Don't change tools if typing in a text field
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.wants_keyboard_input() {
            return;
        }
    }

    if keyboard.just_pressed(KeyCode::KeyV) || keyboard.just_pressed(KeyCode::KeyS) {
        current_tool.tool = EditorTool::Select;
    } else if keyboard.just_pressed(KeyCode::KeyB) || keyboard.just_pressed(KeyCode::KeyP) {
        current_tool.tool = EditorTool::Place;
    } else if keyboard.just_pressed(KeyCode::KeyX) || keyboard.just_pressed(KeyCode::KeyE) {
        current_tool.tool = EditorTool::Erase;
    } else if keyboard.just_pressed(KeyCode::KeyD) {
        current_tool.tool = EditorTool::Draw;
    } else if keyboard.just_pressed(KeyCode::KeyL) {
        current_tool.tool = EditorTool::Line;
    } else if keyboard.just_pressed(KeyCode::KeyT) {
        current_tool.tool = EditorTool::Text;
    }
}

pub fn update_cursor_icon(
    current_tool: Res<CurrentTool>,
    mut window_query: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut commands: Commands,
    mut contexts: EguiContexts,
) {
    // Don't change cursor if over UI
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.is_pointer_over_area() {
            return;
        }
    }

    let Ok((entity, _window)) = window_query.single_mut() else {
        return;
    };

    commands.entity(entity).insert(current_tool.tool.cursor_icon());
}
