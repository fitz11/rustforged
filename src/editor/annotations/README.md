# Annotations Module

Editor-only annotation system for drawing on maps.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports, documentation |
| `components.rs` | Entity components: `DrawnPath`, `DrawnLine`, `TextAnnotation`, `AnnotationMarker` |
| `state.rs` | State resources: `DrawState`, `LineDrawState`, `TextEditState`, `AnnotationSettings` |
| `gizmo.rs` | Custom gizmo group for editor-only rendering |
| `hit_testing.rs` | Hit detection: `point_near_path`, `point_near_line`, `point_in_text`, bounds helpers |
| `layer_helpers.rs` | Layer visibility/locking helpers |
| `draw_tool.rs` | Freehand drawing system |
| `line_tool.rs` | Straight line drawing system |
| `text_tool.rs` | Text annotation system (currently disabled) |
| `rendering.rs` | Gizmo rendering systems for all annotation types |

## Key Types

- **DrawnPath**: Freehand drawing path component (series of connected points)
- **DrawnLine**: Straight line between two points
- **TextAnnotation**: Text label at a specific position
- **AnnotationMarker**: Marker component for all annotation entities
- **AnnotationSettings**: Shared settings (stroke color, width, font size)

## Systems

- **handle_draw**: Handles freehand drawing tool input
- **handle_line**: Handles straight line tool input
- **handle_text**: Handles text annotation tool input (disabled)
- **render_drawn_paths**: Renders all freehand paths via gizmos
- **render_drawn_lines**: Renders all straight lines via gizmos
- **render_draw_preview**: Shows preview while drawing
- **render_line_preview**: Shows preview of line being drawn

## Notes

Annotations are rendered on layer 1 (editor-only) and are not visible in the player view.
