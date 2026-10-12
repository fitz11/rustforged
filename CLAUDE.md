# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rustforged is a D&D 5E Virtual Tabletop (VTT) map editor built with Rust, Bevy 0.20, and bevy_egui. It features layer-based map editing, live session display on secondary monitors, and portable asset libraries.

## Build Commands

```bash
cargo r                    # Dev build with dynamic linking (fast iteration)
cargo rr                   # Optimized release build
cargo run --features dev   # Equivalent to cargo r
cargo run --release        # Equivalent to cargo rr
cargo test                 # Run all tests
cargo test map::layer      # Run tests for specific module
cargo clippy --all-targets -- -D warnings  # Linter (CI enforces this)
```

**Pre-commit validation:**
```bash
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test
```

The `r` and `rr` aliases are defined in `.cargo/config.toml`.

## Architecture

The application uses Bevy's Entity Component System (ECS) with a plugin-based architecture. Plugins are initialized in `main.rs`:

| Plugin | Location | Responsibility | Key Resources |
|--------|----------|----------------|---------------|
| ConfigPlugin | `src/config/` | App settings, persistence | `AppConfig` |
| EditorPlugin | `src/editor/` | Tools, camera, selection, clipboard, history | `CurrentTool`, `SelectedLayer`, `GridSettings`, `Clipboard`, `CommandHistory` |
| AssetLibraryPlugin | `src/assets/` | Asset discovery, thumbnail caching | `AssetLibrary`, `ThumbnailCache`, `SelectedAsset` |
| MapPlugin | `src/map/` | Map data, layers, persistence, fog of war | `MapData`, `FogOfWarData`, `CurrentMapFile`, `MapDirtyState` |
| LiveSessionPlugin | `src/session/` | Player window, viewport controls, freezing and blacking out the player view | `LiveSessionState`, `ViewportDragState`, `PlayerViewport`, `FrozenPlayerView` |
| UiPlugin | `src/ui/` | egui panels, toolbars, dialogs | `DialogState`, `AssetBrowserState` |
| UpdateCheckerPlugin | `src/update/` | Version checking, downloads | `UpdateState` |

### Key Architectural Patterns

**Layer-based rendering with z-ordering** (`src/map/layer.rs`):
- Background (z=0), Terrain (z=50), Doodad (z=100), Token (z=150), GM (z=200), Annotation (z=250), FogOfWar (z=300), Play (z=400)
- Each layer supports up to 25 z-index slots (0-24) within its z-base range
- GM, Annotation, FogOfWar, and Play layers are hidden from the player view

**Message-based communication (Bevy 0.20 `Message` trait):**
```rust
// Define: #[derive(Message)] pub struct SaveMapRequest { pub path: PathBuf }
// Register: app.add_message::<SaveMapRequest>()
// Send: commands.send_message(SaveMapRequest { path })
// Receive: fn system(mut events: MessageReader<SaveMapRequest>) { for e in events.read() { ... } }
// Run condition: .run_if(on_message::<SaveMapRequest>)
```

**Conditional system execution** (`src/editor/conditions.rs`): Systems use `run_if` conditions like `tool_is(EditorTool::Select).and_then(no_dialog_open)`.

**Docked egui panels** (`src/ui/panel_layout.rs`): Show side/top panels with `egui::Panel::...show_docked(ctx, ...)` inside the `EditorPanels` set so each docks against the ones before it. Gate editor mouse input with `is_cursor_over_ui` (`src/editor/params.rs`), not egui's `is_pointer_over_egui`, which doesn't see these panels.

**Mouse input order** (`src/editor/mod.rs`): `PreToolInput` (camera panning, pings) runs first, then the session viewport's handles, then the `ToolInput` set. A click one of these takes is skipped by the later ones (`not_space_panning`, `not_pinging`, `viewport_not_grabbed`).

**Undo and unsaved changes:** Send every map edit through history as a `RecordEditorCommand`, which also marks the map unsaved; history finds objects by their stable `ItemId`, not `Entity`. In egui panels, edit resources through `bypass_change_detection()` and call `set_changed()` only when the user changed something: binding `&mut` every frame makes them count as changed every frame.

**Dialog state aggregation:** `DialogState` resource aggregates all open dialogs in the `First` schedule to block editor input when any modal is open.

**Async I/O with polling:** File operations run in background tasks (`AsyncMapOperation`); main thread polls for completion to prevent UI freezing.

## Module Structure

- `src/input.rs` - Modifier helpers: `command_held` is Ctrl, or Cmd on macOS
- `src/editor/tools.rs` - EditorTool enum (Select, Place, Brush, Draw, Line, Text, Fog, Measure) and `CurrentTool`/`SelectedLayer` resources
- `src/editor/camera.rs` - Panning, zooming toward the cursor, framing with Home
- `src/editor/selection/` - Selection, dragging, resize handles, box select, hit detection
- `src/editor/annotations/` - Drawing, line, text annotation tools and rendering
- `src/editor/history/` - Undo/redo system (`CommandHistory`, `RecordEditorCommand` message; Ctrl+Z / Ctrl+Y). `EditorCommand::{Add, Remove, Change, Fog}` cover every map edit
- `src/editor/fog_overlay.rs` - Fog drawn as a texture, one texel per cell
- `src/editor/markings.rs` - Measure tool (ruler, spell templates) and Alt+click pings; drawn on render layers 0 and 3 so players see them, frozen or not
- `src/map/map_data.rs` - MapData resource, SavedMap serialization
- `src/map/layer.rs` - Layer enum with z-ordering and player visibility
- `src/map/persistence/` - Async save/load with messages (`SaveMapRequest`, `LoadMapRequest`, `NewMapRequest`, `SwitchMapRequest`); `map_state.rs` keeps several maps open (`OpenMaps`); `recovery.rs` autosaves unsaved changes and offers them back after a crash
- `src/ui/map_actions.rs` - New/Open/Save/Close/Quit as `MapFileAction` messages, asking about unsaved changes first
- `src/ui/map_dialogs.rs` - Save As, load/save errors, save conflicts, the recovery offer, the missing-assets report
- `src/ui/shortcuts.rs` - Every shortcut in one table: the Help window shows it, and a test checks README's Controls tables match it (update both together)
- `src/ui/file_drop.rs` - Images and maps dropped on the window from the desktop
- `src/ui/asset_browser/` - Left panel asset management
- `src/ui/layers_panel/` - Right panel: layers, fog, properties, live session controls
- `src/session/player_window.rs` - Secondary monitor player view
- `src/session/freeze.rs` - Frozen player view: copies player-visible items to render layer 3; draw anything players see from `PlayerMapView`, not the live `MapData`/`FogOfWarData`

## Linter Configuration

Certain clippy warnings are intentionally allowed due to Bevy ECS patterns:
- `clippy::too_many_arguments` - Bevy systems often need 10-15+ parameters
- `clippy::type_complexity` - Complex query types are inherent to ECS

## Platform Paths

The `src/paths.rs` module handles platform-appropriate directory resolution:
- Dev mode (cargo run): paths resolve to current directory
- Linux: `~/.config/rustforged/` (config), `~/.local/share/rustforged/` (data)
- Windows/macOS: Platform-specific app data directories

## Debug Logging (Dev Builds Only)

```bash
RUST_LOG=rustforged::editor::selection=trace cargo run  # Trace specific module
RUST_LOG=error cargo run                                 # Errors only
```

Logging is disabled in release builds. Log file: `logs/rustforged.log`
