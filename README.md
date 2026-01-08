# Rustforged

A D&D 5E Virtual Tabletop (VTT) map editor built with Bevy 0.17 and bevy_egui.

## Features

- **Layer-based map editing** - Background, Terrain, Doodad, Token, and Annotation layers with proper z-ordering
- **Layer visibility & locking** - Toggle layer visibility and lock layers to prevent accidental edits
- **Asset library** - Organize terrain tiles, props, and tokens with import support and custom library directories
- **Drawing tools** - Freehand drawing, straight lines, and text annotations
- **Grid system** - 70px grid with snap-to-grid placement (hold Shift for free placement)
- **Map persistence** - Save and load maps as JSON files
- **Live session mode** - Display player view on a secondary monitor with configurable viewport
- **Selection with resize handles** - Resize items using edge and corner handles with direct mouse tracking
- **Context-sensitive toolbars** - Tool-specific settings appear in a secondary toolbar

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd rustforged

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
| Toggle selection | Ctrl + click |
| Box select | Drag on empty space |
| Move selected | Drag selected item or click inside selection |
| Resize selected | Drag edge or corner handles |
| Uniform resize (edges) | Hold Shift while resizing |
| Snap while dragging | Hold Shift |
| Fit to grid | G |
| Delete selected | Delete or Backspace |

### Drawing Tools

| Tool | Usage |
|------|-------|
| Draw | Click and drag to draw freehand paths |
| Line | Click start point, then click end point (right-click to cancel) |
| Text | Click to place text box |

## User Interface

### Main Toolbar

The top toolbar displays between the side panels and includes:
- **Tool buttons** with keyboard shortcut hints (e.g., "Select [V]", "Place [B]")
- **Grid toggle** checkbox
- **Live Session** controls (Start Session button or LIVE indicator)

### Tool Settings Bar

A secondary toolbar appears below the main toolbar for tools with settings:
- **Place tool**: Layer selection dropdown
- **Draw/Line tools**: Color swatches (8 colors) and stroke width
- **Text tool**: Color swatches and font size

### Asset Browser (Left Panel)

- **File/Assets menus** for map and asset operations
- **Asset Library** section with expandable library management
- **Category tabs** (Unsorted, Terrain, Doodads, Tokens)
- **Asset list** with color-coded file type indicators (15x15 preview squares)
- **Selected Asset** section showing:
  - Asset name
  - File type (PNG, JPG, etc.)
  - Native image resolution (e.g., 512x512)

### Layers Panel (Right Panel)

- **Layers** section with visibility and lock controls
- **Properties** section for selected item editing
- **Live Session** controls when session is active

### Cursor Behavior

- **Over UI elements**: Default pointer cursor
- **In editor space**: Tool-specific cursor (crosshair for Place, resize cursors on handles, etc.)

## Project Structure

```
src/
├── main.rs              # App setup, plugin registration
├── assets/              # Asset library management
│   ├── mod.rs           # AssetLibraryPlugin, SelectedAsset
│   ├── asset_type.rs    # AssetCategory enum (Unsorted, Terrain, Doodad, Token)
│   └── library.rs       # AssetLibrary resource, directory management
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
│   ├── selection.rs     # Select/drag/resize/delete items
│   ├── tools.rs         # EditorTool enum, CurrentTool resource
│   └── annotations.rs   # Drawing, lines, text annotations
├── session/             # Live session / player view
│   ├── mod.rs           # LiveSessionPlugin
│   ├── state.rs         # LiveSessionState, viewport config
│   ├── viewport.rs      # Viewport indicator rendering & interaction
│   └── player_window.rs # Secondary window for players
└── ui/                  # egui UI panels
    ├── mod.rs           # UiPlugin
    ├── asset_browser.rs # Left panel - File/Assets menu, library browser
    ├── layers_panel.rs  # Right panel - layers, properties, session controls
    ├── toolbar.rs       # Top toolbars - tools and tool settings
    ├── file_menu.rs     # File operation dialogs
    ├── asset_import.rs  # Asset import dialog
    └── session_controls.rs # Monitor selection dialog
```

## Assets

### Default Library

Assets are loaded from `assets/library/` by default, with subdirectories:

- `unsorted/` - Uncategorized assets
- `terrain/` - Ground tiles, floors, walls
- `doodads/` - Props, furniture, decorations
- `tokens/` - Player/NPC tokens

### Custom Asset Libraries

You can use any directory as an asset library:

1. Click the arrow next to "Asset Library" in the left panel to expand options
2. **Open...** - Select an existing folder with the required subdirectories
3. **New...** - Select a folder to create a new library (subdirectories are created automatically)

When opening an existing library, the folder must contain the required subdirectories (unsorted, terrain, doodads, tokens) or the operation will fail with an error message.

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

### Layer Controls

In the right panel under "Layers":
- **Visibility checkbox** - Show/hide all items on a layer
- **Lock button** - Prevent selection and editing of items on a layer

## Live Session Mode

1. Click "Start Session" in the toolbar
2. Select which monitor to display the player view
3. Use the **move handle** (tab above the viewport) to drag the viewport
4. Resize using corner and edge handles (maintains aspect ratio)
5. Rotate viewport with the rotation buttons in the right panel

The player window displays a fullscreen view of the selected viewport area. The viewport indicator and annotations are only visible in the editor.

## Testing

The project includes unit tests covering core functionality:

```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test map::layer

# Run with verbose output
cargo test -- --nocapture

# Run clippy linter
cargo clippy
```

### Test Coverage

| Module | Tests | Coverage |
|--------|-------|----------|
| `map/layer.rs` | 8 | Layer z-ordering, display names, serialization |
| `assets/asset_type.rs` | 6 | AssetCategory methods, folder paths, serialization |
| `editor/tools.rs` | 10 | EditorTool properties, cursor icons, defaults |
| `session/state.rs` | 26 | Viewport rotation, aspect ratios, bounds calculation |
| `map/map_data.rs` | 18 | MapData defaults, serialization round-trips |
| `editor/grid.rs` | 13 | Grid snapping with various positions and edge cases |
| `session/viewport.rs` | 10 | Point rotation mathematics |
| `map/persistence.rs` | 9 | Color conversion, serialization helpers |

## Dependencies

- Bevy 0.17.3 - Game engine
- bevy_egui 0.38 - Immediate-mode UI
- image 0.25 - Image metadata reading
- serde / serde_json - Serialization
- rfd - Native file dialogs

## License

See LICENSE file for details.
