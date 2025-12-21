use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Layer {
    Background,
    #[default]
    Terrain,
    Doodad,
    Token,
    /// Layer for drawings, lines, and text annotations
    Annotation,
    /// Editor-only layer for the player viewport indicator
    Play,
}

impl Layer {
    pub fn z_base(&self) -> f32 {
        match self {
            Layer::Background => 0.0,
            Layer::Terrain => 100.0,
            Layer::Doodad => 200.0,
            Layer::Token => 300.0,
            Layer::Annotation => 350.0,
            Layer::Play => 400.0,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Layer::Background => "Background",
            Layer::Terrain => "Terrain",
            Layer::Doodad => "Doodads",
            Layer::Token => "Tokens",
            Layer::Annotation => "Annotations",
            Layer::Play => "Play",
        }
    }

    /// Returns all layers available for normal editing (excludes editor-only layers)
    pub fn all() -> &'static [Layer] {
        &[
            Layer::Background,
            Layer::Terrain,
            Layer::Doodad,
            Layer::Token,
            Layer::Annotation,
        ]
    }

    /// Returns true if this layer is editor-only (not visible in player view)
    pub fn is_editor_only(&self) -> bool {
        matches!(self, Layer::Play)
    }
}
