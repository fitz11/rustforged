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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_z_base_ordering() {
        // Z-values should be in ascending order for proper rendering
        assert!(Layer::Background.z_base() < Layer::Terrain.z_base());
        assert!(Layer::Terrain.z_base() < Layer::Doodad.z_base());
        assert!(Layer::Doodad.z_base() < Layer::Token.z_base());
        assert!(Layer::Token.z_base() < Layer::Annotation.z_base());
        assert!(Layer::Annotation.z_base() < Layer::Play.z_base());
    }

    #[test]
    fn test_z_base_values() {
        assert_eq!(Layer::Background.z_base(), 0.0);
        assert_eq!(Layer::Terrain.z_base(), 100.0);
        assert_eq!(Layer::Doodad.z_base(), 200.0);
        assert_eq!(Layer::Token.z_base(), 300.0);
        assert_eq!(Layer::Annotation.z_base(), 350.0);
        assert_eq!(Layer::Play.z_base(), 400.0);
    }

    #[test]
    fn test_display_names() {
        assert_eq!(Layer::Background.display_name(), "Background");
        assert_eq!(Layer::Terrain.display_name(), "Terrain");
        assert_eq!(Layer::Doodad.display_name(), "Doodads");
        assert_eq!(Layer::Token.display_name(), "Tokens");
        assert_eq!(Layer::Annotation.display_name(), "Annotations");
        assert_eq!(Layer::Play.display_name(), "Play");
    }

    #[test]
    fn test_all_excludes_play_layer() {
        let all_layers = Layer::all();
        assert!(!all_layers.contains(&Layer::Play));
    }

    #[test]
    fn test_all_contains_editing_layers() {
        let all_layers = Layer::all();
        assert!(all_layers.contains(&Layer::Background));
        assert!(all_layers.contains(&Layer::Terrain));
        assert!(all_layers.contains(&Layer::Doodad));
        assert!(all_layers.contains(&Layer::Token));
        assert!(all_layers.contains(&Layer::Annotation));
    }

    #[test]
    fn test_all_has_correct_count() {
        assert_eq!(Layer::all().len(), 5);
    }

    #[test]
    fn test_default_is_terrain() {
        assert_eq!(Layer::default(), Layer::Terrain);
    }

    #[test]
    fn test_serialization_roundtrip() {
        for layer in [
            Layer::Background,
            Layer::Terrain,
            Layer::Doodad,
            Layer::Token,
            Layer::Annotation,
            Layer::Play,
        ] {
            let json = serde_json::to_string(&layer).unwrap();
            let deserialized: Layer = serde_json::from_str(&json).unwrap();
            assert_eq!(layer, deserialized);
        }
    }
}
