use bevy::camera::visibility::RenderLayers;
use serde::{Deserialize, Serialize};

/// Render layers that decide which camera draws what. The editor camera sees [`SHARED`]
/// and [`EDITOR`]; the player camera sees [`PLAYER`] plus [`SHARED`], or [`FROZEN`] while
/// the player view is frozen.
///
/// [`SHARED`]: render_layer::SHARED
/// [`EDITOR`]: render_layer::EDITOR
/// [`PLAYER`]: render_layer::PLAYER
/// [`FROZEN`]: render_layer::FROZEN
pub mod render_layer {
    /// Map content that both the GM and the players see
    pub const SHARED: usize = 0;
    /// Only the GM: GM-only items and editor gizmos
    pub const EDITOR: usize = 1;
    /// Only the players: player fog and grid
    pub const PLAYER: usize = 2;
    /// Only the players while frozen: copies of what they saw when the view was frozen
    pub const FROZEN: usize = 3;
}

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
    /// Fog of war: its visibility turns fog on or off for the map
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

    /// Render layers for items on this layer, so GM-only items never reach the player
    /// view.
    pub fn render_layers(&self) -> RenderLayers {
        if self.is_player_visible() {
            RenderLayers::layer(render_layer::SHARED)
        } else {
            RenderLayers::layer(render_layer::EDITOR)
        }
    }

    /// Returns true if this layer is available for use
    pub fn is_available(&self) -> bool {
        true
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
    fn test_render_layers_keep_gm_items_from_players() {
        let shared = RenderLayers::layer(render_layer::SHARED);
        for layer in Layer::all() {
            assert_eq!(
                layer.render_layers().intersects(&shared),
                layer.is_player_visible(),
                "{:?}",
                layer
            );
        }
    }

    #[test]
    fn test_is_available() {
        // All layers are now available
        assert!(Layer::Background.is_available());
        assert!(Layer::Terrain.is_available());
        assert!(Layer::Doodad.is_available());
        assert!(Layer::Token.is_available());
        assert!(Layer::GM.is_available());
        assert!(Layer::Annotation.is_available());
        assert!(Layer::FogOfWar.is_available());
        assert!(Layer::Play.is_available());
    }
}
