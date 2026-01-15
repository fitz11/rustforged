# Clipboard Module

Copy/cut/paste functionality for map items and annotations.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports, documentation |
| `types.rs` | Clipboard data types: `Clipboard`, `ClipboardPlacedItem`, `ClipboardPath`, etc. |
| `helpers.rs` | Utility functions: color conversion, path center calculation, centroid calculation |
| `copy.rs` | `handle_copy` system (Ctrl+C) |
| `cut.rs` | `handle_cut` system (Ctrl+X) |
| `paste.rs` | `handle_paste` system (Ctrl+V) |
| `tests.rs` | Unit tests for clipboard operations |

## Key Types

- **Clipboard**: Resource holding copied items with their offsets from selection centroid
- **ClipboardPlacedItem**: Placed item data for clipboard (asset path, transform, layer)
- **ClipboardPath**: Path annotation data for clipboard
- **ClipboardLine**: Line annotation data for clipboard
- **ClipboardText**: Text annotation data for clipboard

## Systems

- **handle_copy**: Copies selected items to clipboard (Ctrl+C)
- **handle_cut**: Cuts selected items to clipboard (Ctrl+X)
- **handle_paste**: Pastes clipboard items at cursor position (Ctrl+V)

## Notes

Items are stored with their offset from the selection centroid, allowing multiple items
to be pasted while maintaining their relative positions.
