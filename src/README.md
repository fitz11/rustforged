# Source Directory

This is the main source directory for Rustforged. The application is built using a plugin-based architecture with Bevy ECS.

## Files

| File | Purpose |
|------|---------|
| `main.rs` | Application entry point, window configuration, plugin registration |
| `constants.rs` | Global constants (window dimensions, default grid size) |
| `common.rs` | Shared types used across modules (DragMode) |
| `theme.rs` | Color definitions for UI and editor elements |

## Plugin Registration Order

```rust
// From main.rs
app.add_plugins(DefaultPlugins)  // Bevy core
   .add_plugins(EguiPlugin)      // UI framework
   .add_plugins(ConfigPlugin)    // App configuration
   .add_plugins(EditorPlugin)    // Editor systems
   .add_plugins(AssetLibraryPlugin)
   .add_plugins(MapPlugin)
   .add_plugins(LiveSessionPlugin)
   .add_plugins(UiPlugin)
   .add_plugins(UpdateCheckerPlugin);
```

## Module Overview

```
src/
├── assets/    → Asset library management, thumbnails, validation
├── config/    → Application settings persistence
├── editor/    → Tools, selection, camera, annotations, fog
├── map/       → MapData, layers, placed items, save/load
├── session/   → Live session, player window, viewport
├── ui/        → egui panels, dialogs, toolbars
└── update/    → GitHub release checking
```

## Logging Setup

Debug builds configure file-based logging via `tracing_subscriber`:

```rust
// Configured in main.rs setup_logging()
let file_appender = tracing_appender::rolling::never("logs", "rustforged.log");
let env_filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("info,bevy=info,rustforged=debug"));
```

Session markers are written to distinguish between app launches:

```
==================== SESSION START: 2025-01-12 10:30:45 ====================
```

## Common Types

### DragMode

Shared drag interaction mode used by selection and viewport systems:

```rust
pub enum DragMode {
    None,
    Move,
    Rotate,
    ResizeN, ResizeS, ResizeE, ResizeW,
    ResizeNE, ResizeNW, ResizeSE, ResizeSW,
}
```

Each mode maps to an appropriate cursor icon for visual feedback.

## See Also

- [assets/README.md](assets/README.md) - Asset library system
- [editor/README.md](editor/README.md) - Editor tools and systems
- [map/README.md](map/README.md) - Map data and persistence
- [session/README.md](session/README.md) - Live session system
- [ui/README.md](ui/README.md) - User interface panels
