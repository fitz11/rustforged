# Rustforged2

A D&D 5E Virtual Tabletop (VTT) map editor built with Bevy 0.17 and bevy_egui.

## Features

- **Layer-based map editing** - Background, Terrain, Doodad, Token, and Annotation layers with proper z-ordering
- **Asset library** - Organize terrain tiles, props, and tokens with import support
- **Drawing tools** - Freehand drawing, straight lines, and text annotations
- **Grid system** - 70px grid with snap-to-grid placement (hold Shift for free placement)
- **Map persistence** - Save and load maps as JSON files
- **Live session mode** - Display player view on a secondary monitor with configurable viewport

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd rustforged2

# Run the application
cargo run

# Build release version
cargo build --release
```

## Controls

### Camera

| Action | Control |
|--------|---------|
| Pan | Middle-mouse drag |
| Zoom | Scroll wheel (0.1x - 10x) |

### Tool Shortcuts

| Tool | Shortcut |
|------|----------|
| Select | V or S |
| Place | B or P |
| Erase | X or E |
| Draw (freehand) | D |
| Line | L |
| Text | T |

### Selection & Editing

| Action | Control |
|--------|---------|
| Select item | Left-click |
| Multi-select | Shift + drag rectangle |
| Move selected | Drag selected item |
| Snap while dragging | Hold Shift |
| Fit to grid | G |
| Delete selected | Delete or Backspace |

### Drawing Tools

| Tool | Usage |
|------|-------|
| Draw | Click and drag to draw freehand paths |
| Line | Click start point, then click end point (right-click to cancel) |
| Text | Click to place text box |

## Project Structure

```
src/
├── main.rs              # App setup, plugin registration
├── assets/              # Asset library management
│   ├── mod.rs           # AssetLibraryPlugin, SelectedAsset
│   ├── asset_type.rs    # AssetCategory enum (Terrain, Doodad, Token)
│   └── library.rs       # AssetLibrary resource, filesystem scanning
├── map/                 # Map/scene data
│   ├── mod.rs           # MapPlugin
│   ├── layer.rs         # Layer enum with z-ordering
│   ├── placed_item.rs   # PlacedItem, Selected components
│   ├── map_data.rs      # MapData resource, SavedMap format
│   └── persistence.rs   # Save/load/new map systems
├── editor/              # Editor systems
│   ├── mod.rs           # EditorPlugin
│   ├── camera.rs        # Pan/zoom camera
│   ├── grid.rs          # Grid rendering & snap logic
│   ├── placement.rs     # Asset placement on click
│   ├── selection.rs     # Select/drag/delete items
│   ├── tools.rs         # EditorTool enum, CurrentTool resource
│   └── annotations.rs   # Drawing, lines, text annotations
├── session/             # Live session / player view
│   ├── mod.rs           # LiveSessionPlugin
│   ├── state.rs         # LiveSessionState, viewport config
│   ├── viewport.rs      # Viewport indicator rendering
│   └── player_window.rs # Secondary window for players
└── ui/                  # egui UI panels
    ├── mod.rs           # UiPlugin
    ├── asset_browser.rs # Left panel - browse & select assets
    ├── layers_panel.rs  # Right panel - layer visibility/lock
    ├── properties.rs    # Properties window for selected items
    ├── toolbar.rs       # Top toolbar - tools, colors, settings
    ├── file_menu.rs     # File/Assets menus
    ├── asset_import.rs  # Asset import dialog
    └── session_controls.rs # Monitor selection dialog
```

## Assets

Place assets in `assets/library/` with subdirectories:

- `terrain/` - Ground tiles, floors, walls
- `doodads/` - Props, furniture, decorations
- `tokens/` - Player/NPC tokens

Supported formats: PNG, JPG, JPEG, WebP, GIF, BMP, TIFF

Maps are saved to `assets/maps/` as JSON files.

## Layer System

| Layer | Z-Order | Purpose |
|-------|---------|---------|
| Background | 0 | Base terrain, ground |
| Terrain | 100 | Floors, walls, structures |
| Doodad | 200 | Props, furniture, decorations |
| Token | 300 | Player and NPC tokens |
| Annotation | 350 | Drawings, lines, text (editor-only) |
| Play | 400 | Viewport indicator (editor-only) |

## Live Session Mode

1. Click "Start Live Session" in the toolbar
2. Select which monitor to display the player view
3. Drag the viewport rectangle to frame what players see
4. Resize using corner and edge handles
5. Rotate viewport with R/Shift+R for 90-degree increments

The player window displays a fullscreen view of the selected viewport area.

## Dependencies

- Bevy 0.17.3 - Game engine
- bevy_egui 0.38 - Immediate-mode UI
- serde / serde_json - Serialization
- rfd - Native file dialogs

## License

See LICENSE file for details.
