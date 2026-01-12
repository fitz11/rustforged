# Allowed Linter Warnings

This document catalogs all `#[allow(...)]` annotations in the Rustforged codebase, explaining why each warning suppression is necessary and providing guidelines for when new allows are appropriate.

## Summary

| Warning Type | Count | Category |
|--------------|-------|----------|
| `clippy::too_many_arguments` | 21 | Bevy ECS pattern |
| `clippy::type_complexity` | 5 | Complex ECS queries |
| `dead_code` | 17 | Future features / Test utilities |

## Warning Categories

### `clippy::too_many_arguments`

**Why it's allowed:** Bevy ECS systems frequently require many parameters for queries, resources, events, and state. While the typical threshold is 7 parameters, Bevy systems often need 10-15+ parameters to access all required data. Using SystemParam bundles can help but doesn't eliminate all cases.

**Guidelines:**
- Consider using SystemParam bundles when parameters can be logically grouped
- Prefer splitting systems when functionality is clearly separable
- Allow this warning only when the system genuinely needs all parameters

**Locations:**

| File | Line | Function |
|------|------|----------|
| `src/editor/annotations.rs` | 368 | `handle_line` |
| `src/editor/annotations.rs` | 424 | `handle_text` |
| `src/editor/brush.rs` | 49 | `handle_brush` |
| `src/editor/clipboard.rs` | 334 | `handle_paste` |
| `src/editor/fog.rs` | 106 | `handle_fog` |
| `src/editor/history.rs` | 231 | `handle_undo` |
| `src/editor/history.rs` | 267 | `handle_redo` |
| `src/editor/placement.rs` | 12 | `handle_placement` |
| `src/editor/selection/box_select.rs` | 17 | `handle_box_select` |
| `src/editor/selection/cursor.rs` | 15 | `update_selection_cursor` |
| `src/editor/selection/drag.rs` | 13 | `handle_drag` |
| `src/editor/selection/gizmos.rs` | 14 | `draw_selection_indicators` |
| `src/editor/selection/handle.rs` | 21, 240 | `handle_selection`, `start_selection_drag` |
| `src/map/persistence.rs` | 208 | `save_map_system` |
| `src/map/persistence.rs` | 322 | `poll_save_tasks` |
| `src/map/persistence.rs` | 448 | `poll_load_tasks` |
| `src/map/persistence.rs` | 643 | `new_map_system` |
| `src/map/persistence.rs` | 821 | `switch_map_system` |
| `src/ui/asset_browser.rs` | 392 | `asset_browser_ui` |
| `src/ui/layers_panel.rs` | 14 | `layers_panel_ui` |
| `src/ui/mod.rs` | 31 | `update_dialog_state` |

---

### `clippy::type_complexity`

**Why it's allowed:** Complex query types are inherent to Bevy's ECS pattern, especially for queries with multiple component filters. Type aliases can help but add indirection that sometimes hurts readability.

**Guidelines:**
- Consider type aliases for very long types used multiple times
- Use SystemParam bundles to encapsulate related queries
- Allow this warning when the complexity serves clarity

**Locations:**

| File | Line | Item |
|------|------|------|
| `src/editor/camera.rs` | 69 | `apply_camera_zoom` |
| `src/editor/clipboard.rs` | 94 | `calculate_selection_centroid` |
| `src/editor/clipboard.rs` | 132 | `handle_copy` |
| `src/editor/params.rs` | 90 | `SelectedAnnotationQueries` struct |
| `src/editor/selection/drag.rs` | 13 | `handle_drag` |
| `src/editor/selection/gizmos.rs` | 14 | `draw_selection_indicators` |

---

### `dead_code`

**Why it's allowed:** Dead code annotations fall into three categories:
1. **Future features** - Code scaffolding for planned features (documented in TODO.md)
2. **Test utilities** - Methods used only in tests but defined on public types
3. **Backward compatibility** - Fields kept for JSON deserialization of old formats

**Guidelines:**
- Future feature code should be documented in `docs/TODO.md`
- Test-only utilities should be clearly commented
- Remove dead code that serves no purpose
- Never add dead_code allows to hide incomplete work

**Locations:**

#### Future Features (see docs/TODO.md)
| File | Line | Item | Feature |
|------|------|------|---------|
| `src/editor/history.rs` | 25 | `MAX_HISTORY_SIZE` | Undo/Redo system |
| `src/editor/history.rs` | 30 | `EditorCommand` enum | Undo/Redo system |
| `src/editor/history.rs` | 168 | `CommandHistory` impl | Undo/Redo system |
| `src/map/persistence.rs` | 31 | `SwitchMapRequest` | Map switching |
| `src/map/persistence.rs` | 165-203 | Various fields | Map switching UI |

#### Test Utilities
| File | Line | Item | Notes |
|------|------|------|-------|
| `src/common.rs` | 52 | `is_resize()` | Used in tests |
| `src/map/map_data.rs` | 85, 91 | `is_empty()`, `len()` | Used in tests |
| `src/map/fog.rs` | 25, 35, 46 | Fog helper methods | Used in tests |

#### Backward Compatibility
| File | Line | Item | Notes |
|------|------|------|-------|
| `src/map/fog.rs` | 77 | `fogged_cells` field | Legacy format migration |
| `src/map/placed_item.rs` | 22 | `original_path` field | MissingAsset tracking |
| `src/update/mod.rs` | 23 | `name` field | GitHub API response |

---

## Adding New Allows

Before adding a new `#[allow(...)]` annotation:

1. **Try to fix the warning first** - Many warnings have legitimate fixes
2. **Consider refactoring** - Sometimes the warning indicates a design issue
3. **Document the reason** - Add a comment explaining why the allow is necessary
4. **Update this file** - Add the new allow to the appropriate table

### When Allows Are Appropriate

**Do allow:**
- Bevy ECS systems that genuinely need many parameters
- Complex query types that would be less readable as aliases
- Future feature code that's actively planned
- Test utility methods on public types

**Don't allow:**
- Warnings that can be easily fixed
- Dead code with no clear future purpose
- Complexity that indicates a design problem
- Warnings just to make CI pass

---

## Verification

Run these commands to verify the codebase has no unexpected warnings:

```bash
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test
```

All three must pass before changes are complete.
