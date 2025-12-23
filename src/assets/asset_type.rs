use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AssetCategory {
    #[default]
    Unsorted,
    Terrain,
    Doodad,
    Token,
}

impl AssetCategory {
    pub fn folder_name(&self) -> &'static str {
        match self {
            AssetCategory::Unsorted => "unsorted",
            AssetCategory::Terrain => "terrain",
            AssetCategory::Doodad => "doodads",
            AssetCategory::Token => "tokens",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AssetCategory::Unsorted => "unsorted",
            AssetCategory::Terrain => "Terrain",
            AssetCategory::Doodad => "Doodads",
            AssetCategory::Token => "Tokens",
        }
    }

    pub fn all() -> &'static [AssetCategory] {
        &[
            AssetCategory::Unsorted,
            AssetCategory::Terrain,
            AssetCategory::Doodad,
            AssetCategory::Token,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folder_names() {
        assert_eq!(AssetCategory::Unsorted.folder_name(), "unsorted");
        assert_eq!(AssetCategory::Terrain.folder_name(), "terrain");
        assert_eq!(AssetCategory::Doodad.folder_name(), "doodads");
        assert_eq!(AssetCategory::Token.folder_name(), "tokens");
    }

    #[test]
    fn test_display_names() {
        assert_eq!(AssetCategory::Unsorted.display_name(), "unsorted");
        assert_eq!(AssetCategory::Terrain.display_name(), "Terrain");
        assert_eq!(AssetCategory::Doodad.display_name(), "Doodads");
        assert_eq!(AssetCategory::Token.display_name(), "Tokens");
    }

    #[test]
    fn test_all_returns_all_categories() {
        let all = AssetCategory::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&AssetCategory::Unsorted));
        assert!(all.contains(&AssetCategory::Terrain));
        assert!(all.contains(&AssetCategory::Doodad));
        assert!(all.contains(&AssetCategory::Token));
    }

    #[test]
    fn test_default_is_unsorted() {
        assert_eq!(AssetCategory::default(), AssetCategory::Unsorted);
    }

    #[test]
    fn test_serialization_roundtrip() {
        for category in AssetCategory::all() {
            let json = serde_json::to_string(category).unwrap();
            let deserialized: AssetCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(*category, deserialized);
        }
    }

    #[test]
    fn test_folder_names_are_valid_paths() {
        for category in AssetCategory::all() {
            let folder = category.folder_name();
            // Folder names should not contain path separators or special chars
            assert!(!folder.contains('/'));
            assert!(!folder.contains('\\'));
            assert!(!folder.contains(' '));
            assert!(!folder.is_empty());
        }
    }
}
