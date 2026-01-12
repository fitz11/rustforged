# Editor Module

Core editing functionality including tools, camera controls, selection, annotations, and fog of war.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | EditorPlugin, system registration, layer visibility |
| `tools.rs` | EditorTool enum, CurrentTool resource, shortcuts |
| `camera.rs` | Pan/zoom camera, EditorCamera marker, CameraZoom |
| `conditions.rs` | Run conditions (tool_is, session_is_active, no_dialog_open) |
| `params.rs` | SystemParam bundles for complex queries |
| `grid.rs` | Grid rendering and snap_to_grid helper |
| `placement.rs` | Single-click asset placement |
| `brush.rs` | Continuous brush placement |
| `fog.rs` | Fog of war tool and rendering |
| `annotations.rs` | Draw, Line, Text tools and rendering |
| `clipboard.rs` | Copy/cut/paste functionality |
| `history.rs` | Undo/redo infrastructure (future feature) |
| `selection/` | Selection subsystem (see below) |

## Selection Subsystem

```
selection/
├── mod.rs        # Re-exports, DragState, BoxSelectState
├── handle.rs     # Click-to-select, selection logic
├── drag.rs       # Drag selected items
├── box_select.rs # Box selection rectangle
├── shortcuts.rs  # G/C/A/R keys, deletion, escape
├── cursor.rs     # Cursor icon updates
└── gizmos.rs     # Selection indicators, resize handles
```

## Key Types

### EditorTool

```rust
pub enum EditorTool {
    Select,  // Default - select and manipulate items
    Place,   // Single-click placement
    Brush,   // Continuous placement
    Draw,    // Freehand paths
    Line,    // Straight lines
    Text,    // Text annotations
    Fog,     // Fog of war reveal/hide
}
```

### CurrentTool (Resource)

```rust
pub struct CurrentTool {
    pub tool: EditorTool,
}
```

### Tool Keyboard Shortcuts

| Key | Tool/Action |
|-----|-------------|
| V, S | Select |
| P | Place |
| B | Brush |
| D | Draw |
| L | Line |
| T | Text |
| F | Fog |
| C / Shift+C | Cycle layers (Place/Brush) |

## Run Conditions

Used to gate systems based on state:

```rust
// Run only when specific tool is active
.run_if(tool_is(EditorTool::Select))

// Run only when no dialog is open
.run_if(no_dialog_open)

// Run only when live session is active
.run_if(session_is_active)

// Combine conditions
.run_if(tool_is(EditorTool::Place).and(no_dialog_open))
```

## System Flow: Selection

```
Mouse Click
     │
     v
┌────────────────┐
│handle_selection│  Determine click target
└───────┬────────┘
        │
   ┌────┴────┐
   │ Hit?    │
   └────┬────┘
    Yes │ No
   ┌────┴──────────────┐
   v                   v
Select item      Clear selection or
(add Selected)   start box select
        │
        v
┌──────────────────┐
│handle_drag       │  Track drag state
└──────────────────┘
        │
        v
┌──────────────────┐
│draw_selection_   │  Render handles
│indicators        │
└──────────────────┘
```

## Annotations System

Three annotation types with persistent storage:

```rust
pub struct DrawnPath {
    pub points: Vec<Vec2>,
    pub color: Color,
    pub stroke_width: f32,
}

pub struct DrawnLine {
    pub start: Vec2,
    pub end: Vec2,
    pub color: Color,
    pub stroke_width: f32,
}

pub struct TextAnnotation {
    pub content: String,
    pub color: Color,
    pub font_size: f32,
}
```

All annotations are marked with `AnnotationMarker` component and use a dedicated gizmo group for editor-only rendering.

## Fog of War

Fog state and rendering:

```rust
pub struct FogState {
    pub brush_size: f32,      // In grid cells
    pub is_erasing: bool,     // Currently revealing
    pub editor_opacity: f32,  // Opacity in editor view
}
```

Two gizmo groups:
- `FogEditorGizmoGroup` - Semi-transparent grey (RenderLayers::layer(1))
- `FogPlayerGizmoGroup` - Opaque black (RenderLayers::layer(0))

## Grid Snapping

```rust
pub fn snap_to_grid(pos: Vec2, grid_size: f32, center: bool) -> Vec2 {
    if center {
        // Snap to cell center
        let cell_x = (pos.x / grid_size).floor();
        let cell_y = (pos.y / grid_size).floor();
        Vec2::new(
            cell_x * grid_size + grid_size / 2.0,
            cell_y * grid_size + grid_size / 2.0,
        )
    } else {
        // Snap to cell corner
        Vec2::new(
            (pos.x / grid_size).round() * grid_size,
            (pos.y / grid_size).round() * grid_size,
        )
    }
}
```

## SystemParam Bundles

Complex queries are bundled to reduce parameter counts:

```rust
#[derive(SystemParam)]
pub struct CameraParams<'w, 's> {
    pub camera_query: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<EditorCamera>>,
    pub window_query: Query<'w, 's, &'static Window>,
}

impl CameraParams<'_, '_> {
    pub fn cursor_world_pos(&self) -> Option<Vec2> { ... }
}
```

## Code Example: Adding a New Tool

1. Add variant to `EditorTool` in `tools.rs`
2. Implement `display_name()`, `cursor()`, and `keyboard_shortcut()` for variant
3. Create handler system in new file or existing module
4. Register in `EditorPlugin::build()` with appropriate run conditions
5. Add UI button in `toolbar.rs`

```rust
// In tools.rs
pub enum EditorTool {
    // ...existing variants...
    NewTool,
}

// In mod.rs
.add_systems(
    Update,
    new_tool::handle_new_tool
        .run_if(tool_is(EditorTool::NewTool).and(no_dialog_open)),
)
```

## See Also

- [map/README.md](../map/README.md) - PlacedItem, layers
- [ui/README.md](../ui/README.md) - Toolbar, tool settings
- [session/README.md](../session/README.md) - Viewport interactions
