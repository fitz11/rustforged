# Persistence Module

Map save/load operations with async I/O and dirty state tracking.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports, plugin registration |
| `messages.rs` | Message types: `SaveMapRequest`, `LoadMapRequest`, `NewMapRequest` |
| `helpers.rs` | Utility functions: `color_to_array`, `array_to_color`, `ensure_maps_directory` |
| `resources.rs` | `CurrentMapFile`, `OpenMaps`, `OpenMapState` resources |
| `results.rs` | Result types: `MapSaveError`, `MapLoadError`, `SaveMapTask`, `LoadMapTask` |
| `dirty.rs` | Dirty state tracking systems |
| `save.rs` | Save map system and async task handling |
| `load.rs` | Load map system, entity spawning, async task handling |
| `map_state.rs` | Map state capture, `new_map_system`, `switch_map_system` |
| `tests.rs` | Unit tests for persistence operations |

## Key Types

- **CurrentMapFile**: Resource tracking the current map's file path
- **OpenMaps**: Resource tracking all open maps and their state
- **MapDirtyState**: Resource tracking whether map has unsaved changes
- **SaveMapTask**: Async task component for save operations
- **LoadMapTask**: Async task component for load operations

## Systems

- **save_map_system**: Handles save requests, initiates async save
- **poll_save_tasks**: Polls async save tasks for completion
- **load_map_system**: Handles load requests, initiates async load
- **poll_load_tasks**: Polls async load tasks, spawns loaded entities
- **new_map_system**: Creates a new empty map
- **switch_map_system**: Switches between open maps
