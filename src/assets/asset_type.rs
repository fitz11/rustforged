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
