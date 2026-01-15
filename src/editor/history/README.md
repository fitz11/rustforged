# History Module

Undo/redo command history system for editor operations.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports, `MAX_HISTORY_SIZE`, `EditorCommand` enum |
| `data_types.rs` | Snapshot types: `PlacedItemData`, `TransformData`, `PathData`, `LineData`, `TextData` |
| `command_history.rs` | `CommandHistory` resource |
| `commands.rs` | `EditorCommand` enum with all undo/redo-able operations |
| `spawn_helpers.rs` | Helper functions for spawning entities from data snapshots |
| `execute.rs` | `execute_undo`, `execute_redo` implementation |
| `systems.rs` | Bevy systems: `handle_undo`, `handle_redo` |
| `tests.rs` | Unit tests for command history operations |

## Key Types

- **CommandHistory**: Resource storing undo/redo stacks with size limit
- **EditorCommand**: Enum of all commands that can be undone/redone
- **PlacedItemData**: Snapshot of a placed item for undo/redo
- **PathData**: Snapshot of a drawn path annotation
- **LineData**: Snapshot of a drawn line annotation
- **TextData**: Snapshot of a text annotation

## Systems

- **handle_undo**: Keyboard handler for Ctrl+Z (undo)
- **handle_redo**: Keyboard handler for Ctrl+Y / Ctrl+Shift+Z (redo)
