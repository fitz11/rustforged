# Assets Module

Manages asset libraries, thumbnails, and asset validation for the map editor.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | AssetLibraryPlugin, SelectedAsset resource, message handlers |
| `library.rs` | AssetLibrary resource, directory scanning, LibraryAsset |
| `asset_type.rs` | AssetCategory enum (Unsorted, Terrain, Doodad, Token) |
| `placeholder.rs` | Placeholder texture for missing assets |
| `validation.rs` | MissingAsset detection and indicator rendering |

## Key Types

### AssetLibrary (Resource)

Main resource tracking the current asset library:

```rust
pub struct AssetLibrary {
    pub library_path: PathBuf,     // Root directory
    pub assets: Vec<LibraryAsset>, // All discovered assets
    pub metadata: LibraryMetadata, // Library name from .library.json
}
```

### LibraryAsset

Represents a single asset in the library:

```rust
pub struct LibraryAsset {
    pub name: String,           // Filename without extension
    pub relative_path: String,  // Path relative to library root
    pub category: AssetCategory,
    pub file_type: String,      // Extension (png, jpg, etc.)
}
```

### SelectedAsset (Resource)

Tracks the currently selected asset in the browser:

```rust
pub struct SelectedAsset {
    pub asset: Option<LibraryAsset>,
}
```

### AssetCategory

```rust
pub enum AssetCategory {
    Unsorted,  // unsorted/
    Terrain,   // terrain/
    Doodad,    // doodads/
    Token,     // tokens/
}
```

## Data Flow

```
Library Directory
       │
       v
┌─────────────────┐
│ scan_asset_lib  │  Discover all image files
└────────┬────────┘
         │
         v
┌─────────────────┐
│ AssetLibrary    │  Store in resource
│ .assets[]       │
└────────┬────────┘
         │
    ┌────┴────┐
    v         v
┌────────┐ ┌──────────────┐
│Thumbnail│ │ Asset Browser│
│ Cache   │ │ UI Display   │
└─────────┘ └──────────────┘
```

## Messages

| Message | Purpose |
|---------|---------|
| `RefreshAssetLibrary` | Trigger rescan of library directory |
| `UpdateLibraryMetadataRequest` | Update library name in .library.json |
| `RenameAssetRequest` | Rename asset and update placed items |

## Thumbnail System

Thumbnails are generated lazily and cached:

```rust
pub struct ThumbnailCache {
    pub thumbnails: HashMap<String, TextureId>,
}

pub const THUMBNAIL_SIZE: u32 = 64;
```

The `load_and_register_thumbnails` system processes a few thumbnails per frame to avoid hitches.

## Validation System

Detects and marks placed items whose assets no longer exist:

```rust
pub struct MissingAsset {
    pub original_path: String,
}
```

Missing assets display a red X indicator via the `draw_missing_asset_indicators` gizmo system.

## Library Directory Structure

```
my_library/
├── .library.json      # Metadata (name)
├── unsorted/          # Uncategorized assets
├── terrain/           # Ground, floors, walls
├── doodads/           # Props, furniture
├── tokens/            # Characters, creatures
└── maps/              # Saved map files
```

## Code Examples

### Opening a Library

```rust
fn open_library_directory(
    library: &mut AssetLibrary,
    path: PathBuf,
) -> Result<(), String> {
    // Validate required subdirectories exist
    for category in AssetCategory::all() {
        let subdir = path.join(category.folder_name());
        if !subdir.exists() {
            return Err(format!("Missing folder: {}", category.folder_name()));
        }
    }
    library.library_path = path;
    Ok(())
}
```

### Adding a New Asset Category

1. Add variant to `AssetCategory` enum in `asset_type.rs`
2. Implement `folder_name()` and `display_name()` for the variant
3. Create the corresponding subdirectory in library creation

## See Also

- [ui/README.md](../ui/README.md) - Asset browser UI
- [map/README.md](../map/README.md) - PlacedItem uses asset paths
