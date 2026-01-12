# Map Module

Handles map data, layers, placed items, fog of war, and persistence (save/load).

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | MapPlugin, coordinate helpers, cell functions |
| `map_data.rs` | MapData resource, SavedMap format, asset manifest |
| `layer.rs` | Layer enum with z-ordering |
| `placed_item.rs` | PlacedItem component, Selected marker |
| `fog.rs` | FogOfWarData resource, cell-based fog storage |
| `persistence.rs` | Save/load systems, async operations |

## Key Types

### MapData (Resource)

Core map configuration:

```rust
pub struct MapData {
    pub name: String,
    pub grid_size: f32,      // Default: 70.0
    pub grid_visible: bool,
    pub layers: Vec<LayerData>,
}

pub struct LayerData {
    pub layer_type: Layer,
    pub visible: bool,
    pub locked: bool,
}
```

### Layer

```rust
pub enum Layer {
    Background,  // z: 0
    Terrain,     // z: 100
    Doodad,      // z: 200
    Token,       // z: 300
    Annotation,  // z: 350
    FogOfWar,    // z: 375
    Play,        // z: 400
}
```

Use `layer.z_base()` to get the base z-order, then add `z_index` for fine control.

### PlacedItem (Component)

```rust
pub struct PlacedItem {
    pub asset_path: String,  // Relative to library
    pub layer: Layer,
    pub z_index: i32,        // Within-layer ordering
}
```

### FogOfWarData (Resource)

Cell-based fog storage:

```rust
pub struct FogOfWarData {
    pub revealed_cells: HashSet<(i32, i32)>,
}

impl FogOfWarData {
    pub fn is_cell_revealed(&self, cell: (i32, i32)) -> bool;
    pub fn reveal_cell(&mut self, cell: (i32, i32));
}
```

## SavedMap Format (JSON)

```json
{
  "asset_manifest": {
    "assets": ["terrain/grass.png", "tokens/hero.png"]
  },
  "map_data": {
    "name": "Dungeon Level 1",
    "grid_size": 70.0,
    "grid_visible": true,
    "layers": [...]
  },
  "placed_items": [
    {
      "asset_path": "terrain/grass.png",
      "position": [100.0, 200.0],
      "rotation": 0.0,
      "scale": [1.0, 1.0],
      "layer": "Terrain",
      "z_index": 0
    }
  ],
  "annotations": {
    "paths": [...],
    "lines": [...],
    "text_boxes": [...]
  },
  "fog_of_war": {
    "revealed_cells": [[0, 0], [1, 0], [0, 1]]
  }
}
```

## Coordinate Helpers

```rust
// World position to grid cell
pub fn world_to_cell(pos: Vec2, grid_size: f32) -> (i32, i32) {
    (
        (pos.x / grid_size).floor() as i32,
        (pos.y / grid_size).floor() as i32,
    )
}

// Grid cell to world center
pub fn cell_to_world(cell: (i32, i32), grid_size: f32) -> Vec2 {
    Vec2::new(
        cell.0 as f32 * grid_size + grid_size / 2.0,
        cell.1 as f32 * grid_size + grid_size / 2.0,
    )
}

// Get all cells within radius of a point
pub fn cells_in_radius(center: Vec2, radius: f32, grid_size: f32) -> Vec<(i32, i32)>;
```

## Save/Load Flow

```
SaveMapRequest
      │
      v
┌─────────────────┐
│ Collect items,  │
│ annotations,    │
│ fog data        │
└────────┬────────┘
         │
         v
┌─────────────────┐
│ Serialize to    │
│ SavedMap JSON   │
└────────┬────────┘
         │
         v
┌─────────────────┐
│ Spawn async     │
│ write task      │
└────────┬────────┘
         │
         v
┌─────────────────┐
│ poll_save_tasks │  Check completion
└────────┬────────┘
         │
         v
Update dirty state,
show error if failed
```

## Messages

| Message | Purpose |
|---------|---------|
| `SaveMapRequest` | Save map to specified path |
| `LoadMapRequest` | Load map from specified path |
| `NewMapRequest` | Create new empty map |
| `SwitchMapRequest` | Switch to different open map (future) |

## Validation Resources

```rust
pub struct SaveValidationWarning {
    pub show: bool,
    pub missing_assets: Vec<String>,
    pub pending_save_path: Option<PathBuf>,
}

pub struct LoadValidationWarning {
    pub show: bool,
    pub missing_assets: Vec<String>,
    pub map_path: Option<PathBuf>,
}
```

## Code Example: Adding a New Layer

1. Add variant to `Layer` enum in `layer.rs`
2. Implement `z_base()`, `display_name()`, `is_player_visible()`, `is_available()`
3. Update `Layer::all()` to include new variant
4. Add UI controls in `layers_panel.rs` if needed

```rust
// In layer.rs
pub enum Layer {
    // ...existing...
    NewLayer,
}

impl Layer {
    pub fn z_base(&self) -> f32 {
        match self {
            // ...existing...
            Layer::NewLayer => 250.0,  // Between Doodad and Token
        }
    }
}
```

## See Also

- [editor/README.md](../editor/README.md) - Selection, placement
- [session/README.md](../session/README.md) - Player view filtering
- [ui/README.md](../ui/README.md) - Layers panel
