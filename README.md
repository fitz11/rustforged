# Rustforged

A D&D 5E Virtual Tabletop (VTT) map editor built with Bevy 0.17 and bevy_egui.

## Project Goals

Rustforged aims to be a lightweight, performant VTT map editor focused on:

- **Layer-based map creation** - Intuitive terrain, doodad, and token placement with proper z-ordering
- **Live session display** - Show player-facing view on a secondary monitor with fog of war
- **Portable asset libraries** - Organize and reuse assets across multiple maps
- **Minimal dependencies** - Built with Rust and Bevy for native performance without browser overhead

## Features

- **Layer-based map editing** - Background, Terrain, Doodad, Token, Annotation, and FogOfWar layers
- **Layer visibility & locking** - Toggle layer visibility and lock layers to prevent accidental edits
- **Asset library management** - Create, open, and organize custom asset libraries
- **Drawing tools** - Freehand drawing, straight lines, and text annotations
- **Fog of War** - Reveal/hide map areas for players with circular brush or grid-aligned tools
- **Grid system** - 70px grid with snap-to-grid placement (hold Shift for free placement)
- **Map persistence** - Save and load maps as JSON files
- **Live session mode** - Display player view on a secondary monitor with configurable viewport
- **Selection with resize handles** - Resize items using edge and corner handles

## Installation

Rustforged is built from source. You'll need the Rust toolchain installed.

### Prerequisites

1. **Install Rust** via [rustup](https://rustup.rs/):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   On Windows, download and run [rustup-init.exe](https://win.rustup.rs/).

2. **System dependencies** (Linux only):
   ```bash
   # Debian/Ubuntu
   sudo apt install libasound2-dev libudev-dev pkg-config

   # Fedora
   sudo dnf install alsa-lib-devel systemd-devel

   # Arch
   sudo pacman -S alsa-lib systemd-libs
   ```

### Build & Run

```bash
# Clone the repository
git clone https://github.com/fitz11/rustforged.git
cd rustforged

# Run in development mode
cargo run

# Or build an optimized release version
cargo build --release
# Binary will be at target/release/rustforged (or rustforged.exe on Windows)
```

## Controls

### Camera

| Action | Control |
|--------|---------|
| Pan | Middle-mouse drag |
| Zoom | Scroll wheel (0.1x - 10x) |

### Tool Shortcuts

| Tool | Shortcut | Description |
|------|----------|-------------|
| Select | V or S | Select, move, and resize items |
| Place | P | Single-click asset placement |
| Brush | B | Continuous placement while dragging |
| Draw | D | Freehand drawing paths |
| Line | L | Straight lines between two points |
| Text | T | Text annotations |
| Fog | F | Reveal/hide fog of war areas |
| Cycle Layer | C / Shift+C | Switch between layers (Place/Brush tools) |

### Selection & Editing

| Action | Control |
|--------|---------|
| Select item | Left-click |
| Toggle selection | Ctrl + click |
| Box select | Drag on empty space |
| Move selected | Drag selected item or click inside selection |
| Resize selected | Drag edge or corner handles |
| Snap while dragging | Hold Shift |
| Fit to grid | G |
| Center to grid | C |
| Restore aspect ratio | A |
| Rotate 90 degrees | R |
| Delete selected | Delete or Backspace |
| Copy | Ctrl+C |
| Cut | Ctrl+X |
| Paste | Ctrl+V |

### Drawing Tools

| Tool | Usage |
|------|-------|
| Draw | Click and drag to draw freehand paths |
| Line | Click start point, then click end point (right-click to cancel) |
| Text | Click to place text box |

### Fog of War

| Action | Control |
|--------|---------|
| Reveal (circular brush) | Click and drag |
| Reveal (single cell) | Shift + click |

## Architecture

Rustforged uses a plugin-based architecture built on the Bevy Entity Component System (ECS):

```
               +-------------+
               |   main.rs   |
               +------+------+
                      |
    +---------+-------+-------+---------+
    |         |       |       |         |
+---v---+ +---v---+ +-v--+ +--v--+ +----v----+
| Asset | | Editor| | Map| | UI  | | Session |
| Lib   | | Plugin| |Data| |Plugin| | Plugin  |
+-------+ +-------+ +----+ +-----+ +---------+
```

**Plugin responsibilities:**

- **AssetLibraryPlugin** - Asset discovery, thumbnail caching, library management
- **EditorPlugin** - Camera, tools, selection, annotations, fog of war
- **MapPlugin** - Map data, layers, placed items, persistence
- **UiPlugin** - egui panels, toolbars, dialogs
- **LiveSessionPlugin** - Player window, viewport controls

## Logging (Debug Builds)

Debug builds include file-based logging for troubleshooting:

- **Log file**: `logs/rustforged.log`
- **Default levels**: `info` for Bevy, `debug` for rustforged
- **Custom filtering**: Set `RUST_LOG` environment variable

```bash
# Example: Enable trace logging for selection module
RUST_LOG=rustforged::editor::selection=trace cargo run

# Example: Quiet mode (errors only)
RUST_LOG=error cargo run
```

**Note**: Logging is disabled in release builds for performance.

## User Interface

### Main Toolbar

The top toolbar displays between the side panels and includes:
- **Tool buttons** with keyboard shortcut hints (e.g., "Select [V]", "Place [P]")
- **Grid toggle** checkbox
- **Live Session** controls (Start Session button or LIVE indicator)

### Tool Settings Bar

A secondary toolbar appears below the main toolbar for tools with settings:
- **Place/Brush tools**: Layer selection dropdown (use L/Shift+L to cycle)
- **Draw/Line tools**: Color swatches and stroke width
- **Text tool**: Color swatches and font size
- **Fog tool**: Brush size slider and opacity control

### Asset Browser (Left Panel)

- **File/Assets menus** for map and asset operations
- **Asset Library** section with expandable library management
- **Category tabs** (Unsorted, Terrain, Doodads, Tokens)
- **Asset list** with thumbnail previews
- **Selected Asset** section with Rename and Move buttons

| Shortcut | Action |
|----------|--------|
| F2 | Rename selected asset |
| F3 | Rename current map |
| F4 | Rename library |

### Layers Panel (Right Panel)

- **Layers** section with visibility and lock controls
- **Properties** section for selected item editing
- **Live Session** controls when session is active

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

Supported formats: PNG, JPG, JPEG, WebP, GIF, BMP, TIFF

Maps are saved to `<library>/maps/` as JSON files.

## Layer System

| Layer | Z-Order | Purpose |
|-------|---------|---------|
| Background | 0 | Base terrain, ground |
| Terrain | 100 | Floors, walls, structures |
| Doodad | 200 | Props, furniture, decorations |
| Token | 300 | Player and NPC tokens |
| Annotation | 350 | Drawings, lines, text (editor-only) |
| FogOfWar | 375 | Fog overlay (different in editor vs player) |
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

The player window displays a fullscreen view of the selected viewport area. The viewport indicator and annotations are only visible in the editor. Fog of War appears as black on the player view.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| bevy | 0.17.3 | Game engine (rendering, ECS, windowing) |
| bevy_egui | 0.38 | Immediate-mode UI integration |
| image | 0.25 | Image metadata reading |
| serde / serde_json | 1.x | JSON serialization |
| rfd | 0.15 | Native file dialogs |
| ureq | 2.x | HTTP client for update checking |
| tracing-subscriber | 0.3 | Logging framework |
| chrono | 0.4 | Timestamps in logs |

## Testing

```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test map::layer

# Run clippy linter
cargo clippy

# Pre-commit checks (same as CI)
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test
```

### Test Coverage

| Module | Tests | Coverage |
|--------|-------|----------|
| `map/layer.rs` | 8 | Layer z-ordering, display names, serialization |
| `assets/asset_type.rs` | 6 | AssetCategory methods, folder paths |
| `editor/tools.rs` | 10 | EditorTool properties, cursor icons |
| `session/state.rs` | 26 | Viewport rotation, aspect ratios |
| `map/map_data.rs` | 18 | MapData defaults, serialization |
| `editor/grid.rs` | 13 | Grid snapping edge cases |
| `session/viewport.rs` | 10 | Point rotation mathematics |
| `map/persistence.rs` | 9 | Color conversion, serialization |

## Contributing

See [docs/ALLOWED_WARNINGS.md](docs/ALLOWED_WARNINGS.md) for linter configuration and [docs/TODO.md](docs/TODO.md) for planned features.

## License

MIT OR Apache-2.0
