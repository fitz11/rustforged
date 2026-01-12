use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Layer {
    Background,
    #[default]
    Terrain,
    Doodad,
    Token,
    /// GM-only layer, hidden from player view
    GM,
    /// Layer for drawings, lines, and text annotations (editor-only)
    Annotation,
    /// Fog of war layer - reserved for future implementation
    FogOfWar,
    /// Editor-only layer for the player viewport indicator
    Play,
}

impl Layer {
    pub fn z_base(&self) -> f32 {
        match self {
            Layer::Background => 0.0,
            Layer::Terrain => 50.0,
            Layer::Doodad => 100.0,
            Layer::Token => 150.0,
            Layer::GM => 200.0,
            Layer::Annotation => 250.0,
            Layer::FogOfWar => 300.0,
            Layer::Play => 400.0,
        }
    }

    /// Maximum z-index value allowed within a layer (0 to max_z_index inclusive)
    pub fn max_z_index() -> i32 {
        24
    }

    /// Returns true if this layer should be visible to players in live sessions
    pub fn is_player_visible(&self) -> bool {
        !matches!(
            self,
            Layer::GM | Layer::Annotation | Layer::FogOfWar | Layer::Play
        )
    }

    /// Returns true if this layer is available for use (not reserved for future implementation)
    pub fn is_available(&self) -> bool {
        !matches!(self, Layer::FogOfWar)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Layer::Background => "Background",
            Layer::Terrain => "Terrain",
            Layer::Doodad => "Doodads",
            Layer::Token => "Tokens",
            Layer::GM => "GM",
            Layer::Annotation => "Annotations",
            Layer::FogOfWar => "Fog of War",
            Layer::Play => "Play",
        }
    }

    /// Returns all layers available for normal editing (excludes editor-only layers like Play)
    pub fn all() -> &'static [Layer] {
        &[
            Layer::Background,
            Layer::Terrain,
            Layer::Doodad,
            Layer::Token,
            Layer::GM,
            Layer::Annotation,
            Layer::FogOfWar,
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
        assert!(Layer::Token.z_base() < Layer::GM.z_base());
        assert!(Layer::GM.z_base() < Layer::Annotation.z_base());
        assert!(Layer::Annotation.z_base() < Layer::FogOfWar.z_base());
        assert!(Layer::FogOfWar.z_base() < Layer::Play.z_base());
    }

    #[test]
    fn test_z_base_values() {
        assert_eq!(Layer::Background.z_base(), 0.0);
        assert_eq!(Layer::Terrain.z_base(), 50.0);
        assert_eq!(Layer::Doodad.z_base(), 100.0);
        assert_eq!(Layer::Token.z_base(), 150.0);
        assert_eq!(Layer::GM.z_base(), 200.0);
        assert_eq!(Layer::Annotation.z_base(), 250.0);
        assert_eq!(Layer::FogOfWar.z_base(), 300.0);
        assert_eq!(Layer::Play.z_base(), 400.0);
    }

    #[test]
    fn test_display_names() {
        assert_eq!(Layer::Background.display_name(), "Background");
        assert_eq!(Layer::Terrain.display_name(), "Terrain");
        assert_eq!(Layer::Doodad.display_name(), "Doodads");
        assert_eq!(Layer::Token.display_name(), "Tokens");
        assert_eq!(Layer::GM.display_name(), "GM");
        assert_eq!(Layer::Annotation.display_name(), "Annotations");
        assert_eq!(Layer::FogOfWar.display_name(), "Fog of War");
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
        assert!(all_layers.contains(&Layer::GM));
        assert!(all_layers.contains(&Layer::Annotation));
        assert!(all_layers.contains(&Layer::FogOfWar));
    }

    #[test]
    fn test_all_has_correct_count() {
        assert_eq!(Layer::all().len(), 7);
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
            Layer::GM,
            Layer::Annotation,
            Layer::FogOfWar,
            Layer::Play,
        ] {
            let json = serde_json::to_string(&layer).unwrap();
            let deserialized: Layer = serde_json::from_str(&json).unwrap();
            assert_eq!(layer, deserialized);
        }
    }

    #[test]
    fn test_max_z_index() {
        assert_eq!(Layer::max_z_index(), 24);
    }

    #[test]
    fn test_is_player_visible() {
        // Player-visible layers
        assert!(Layer::Background.is_player_visible());
        assert!(Layer::Terrain.is_player_visible());
        assert!(Layer::Doodad.is_player_visible());
        assert!(Layer::Token.is_player_visible());

        // Not visible to players
        assert!(!Layer::GM.is_player_visible());
        assert!(!Layer::Annotation.is_player_visible());
        assert!(!Layer::FogOfWar.is_player_visible());
        assert!(!Layer::Play.is_player_visible());
    }

    #[test]
    fn test_is_available() {
        // Available layers
        assert!(Layer::Background.is_available());
        assert!(Layer::Terrain.is_available());
        assert!(Layer::Doodad.is_available());
        assert!(Layer::Token.is_available());
        assert!(Layer::GM.is_available());
        assert!(Layer::Annotation.is_available());
        assert!(Layer::Play.is_available());

        // Reserved layer
        assert!(!Layer::FogOfWar.is_available());
    }
}
