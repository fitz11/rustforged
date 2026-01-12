# Future Features

This document tracks planned features that have scaffolding code in the codebase. These features have `#[allow(dead_code)]` annotations that should be kept until the features are fully implemented.

## Undo/Redo System

**Status:** Scaffolded, needs integration

**Location:** `src/editor/history.rs`

**Description:**
A command-based undo/redo system that records all reversible editor operations (placement, deletion, movement, annotations) and allows users to undo/redo them.

**Keyboard Shortcuts:**
- `Ctrl+Z` - Undo last action
- `Ctrl+Y` or `Ctrl+Shift+Z` - Redo last undone action

**Current State:**
- `EditorCommand` enum defines all command types
- `CommandHistory` resource with undo/redo stacks implemented
- `handle_undo` and `handle_redo` systems registered
- Helper functions for spawning entities from command data

**What's Missing:**
- Integration with placement/deletion systems to push commands
- Integration with transform changes to record moves
- Integration with annotation creation/deletion
- UI indicators showing undo/redo availability
- Menu items for undo/redo

**Dead Code Items:**
| Item | Line | Purpose |
|------|------|---------|
| `MAX_HISTORY_SIZE` | 25 | Limits history to 100 commands |
| `EditorCommand` | 30 | All command variants |
| `CommandHistory` impl | 168 | History manipulation methods |

**To Complete:**
1. In `placement.rs`, after spawning an entity, push a `PlaceItems` command
2. In `selection/handle.rs`, after deletion, push a `DeleteItems` command
3. In drag handling, push `MoveItems` commands with before/after transforms
4. Similar integration for annotation tools
5. Add undo/redo count to Edit menu or toolbar

---

## Multi-Map Support / Map Switching

**Status:** Partially scaffolded

**Location:** `src/map/persistence.rs`

**Description:**
Allow users to have multiple maps open simultaneously and switch between them. The tab-based UI would show open maps and allow switching without losing unsaved changes.

**Current State:**
- `OpenMaps` resource tracks multiple open maps by ID
- `OpenMap` struct holds per-map state (name, path, dirty flag, saved state)
- `SwitchMapRequest` message defined
- `switch_map_system` partially implemented
- `UnsavedChangesDialog` has fields for switch confirmation

**What's Missing:**
- Tab bar UI to show open maps
- Proper state serialization/restoration when switching
- "Close map" functionality with unsaved changes prompt
- Integration with existing save/load to update OpenMaps state
- Keyboard shortcuts for switching maps (Ctrl+Tab?)

**Dead Code Items:**
| Item | Line | Purpose |
|------|------|---------|
| `SwitchMapRequest` | 31 | Message to trigger map switch |
| `active_map()` | 165 | Get currently active map |
| `has_any_unsaved()` | 176 | Check for any unsaved maps |
| `unsaved_maps()` | 182 | List maps with changes |
| `show_switch_confirmation` | 192 | Dialog flag for switch warning |
| `pending_switch_id` | 195 | Target map for switch |
| `show_load_confirmation` | 200 | Dialog for load with unsaved |
| `pending_load_path` | 203 | Path to load after confirm |

**To Complete:**
1. Add tab bar UI component showing open maps
2. Implement full state save/restore for map switching
3. Handle unsaved changes when switching maps
4. Add "New Tab" / "Close Tab" functionality
5. Keyboard shortcuts for navigation

---

## Fog of War Helper Methods

**Status:** Implemented but methods unused

**Location:** `src/map/fog.rs`

**Description:**
Helper methods for fog-of-war manipulation that may be useful for future features like bulk fog operations or scripting.

**Dead Code Items:**
| Item | Line | Purpose |
|------|------|---------|
| `reveal_all()` | 25 | Clear all fog from map |
| `is_cell_fogged()` | 35 | Check if specific cell is fogged |
| `fog_cell()` | 46 | Add fog to specific cell |
| `fogged_cells` field | 77 | Legacy format for backwards compat |

**Note:** These methods are kept for potential use in:
- "Reset Fog" / "Clear Fog" toolbar buttons
- Scripted fog manipulation
- Import/export of fog state

---

## Implementation Priority

1. **Undo/Redo** - High value, improves usability significantly
2. **Map Switching** - Medium value, useful for complex sessions
3. **Fog Helpers** - Low priority, already have UI controls

## Contributing

When implementing these features:
1. Remove the `#[allow(dead_code)]` annotations as code becomes used
2. Update this document to reflect progress
3. Add tests for new functionality
4. Update `docs/ALLOWED_WARNINGS.md` if allows change
