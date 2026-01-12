# Session Module

Manages live session display for showing the map to players on a secondary monitor.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | LiveSessionPlugin, resource initialization, system registration |
| `state.rs` | LiveSessionState, ViewportDragState, MonitorInfo |
| `viewport.rs` | Viewport indicator rendering, resize/move handles |
| `player_window.rs` | Secondary window creation, camera sync |

## Key Types

### LiveSessionState (Resource)

Main resource tracking the live session:

```rust
pub struct LiveSessionState {
    pub is_active: bool,
    pub selected_monitor: Option<MonitorInfo>,
    pub viewport_center: Vec2,
    pub viewport_size: Vec2,
    pub rotation_degrees: i32,  // 0, 90, 180, 270
}

impl LiveSessionState {
    pub fn monitor_aspect_ratio(&self) -> f32;
    pub fn rotation_radians(&self) -> f32;
    pub fn rotate_cw(&mut self);   // +90 degrees
    pub fn rotate_ccw(&mut self);  // -90 degrees
    pub fn is_portrait(&self) -> bool;
    pub fn effective_viewport_size(&self) -> Vec2;  // Swaps w/h for 90/270
}
```

### MonitorInfo

Information about a connected display:

```rust
pub struct MonitorInfo {
    pub name: String,
    pub physical_size: UVec2,
    pub index: usize,
}

impl MonitorInfo {
    pub fn aspect_ratio(&self) -> f32;
}
```

### ViewportDragState (Resource)

Tracks drag interactions with the viewport indicator:

```rust
pub struct ViewportDragState {
    pub mode: ViewportDragMode,  // Move, ResizeN/S/E/W, ResizeNE/NW/SE/SW
    pub drag_start_world: Vec2,
    pub original_center: Vec2,
    pub original_size: Vec2,
}
```

### Marker Components

```rust
pub struct PlayerWindow;   // Marks the player display window
pub struct PlayerCamera;   // Marks the camera rendering to player window
```

## Dual-Window Architecture

```
┌─────────────────────────────────────────┐
│           Primary Window                 │
│         (Editor View)                    │
│                                          │
│  ┌──────────────────────┐               │
│  │    Map Canvas        │               │
│  │                      │               │
│  │   ┌──────────┐       │  Viewport     │
│  │   │ Viewport │◄──────┼─ indicator    │
│  │   │ Indicator│       │  shows player │
│  │   └──────────┘       │  view bounds  │
│  │                      │               │
│  └──────────────────────┘               │
└─────────────────────────────────────────┘
                │
                │ sync_player_camera
                ▼
┌─────────────────────────────────────────┐
│           Player Window                  │
│     (Secondary Monitor/Fullscreen)       │
│                                          │
│  ┌──────────────────────┐               │
│  │                      │               │
│  │   Player sees only   │               │
│  │   RenderLayers::0    │               │
│  │   (no annotations,   │               │
│  │    no viewport box)  │               │
│  │                      │               │
│  └──────────────────────┘               │
└─────────────────────────────────────────┘
```

## Render Layers

The system uses Bevy's `RenderLayers` to control what each camera sees:

| Layer | Contents | Editor Camera | Player Camera |
|-------|----------|---------------|---------------|
| 0 | Map content (terrain, tokens, fog) | Yes | Yes |
| 1 | Editor-only (annotations, viewport indicator) | Yes | No |

## Session Lifecycle

```
Start Session
     │
     v
┌─────────────────┐
│ Select monitor  │  MonitorSelectionDialog
└────────┬────────┘
         │
         v
┌─────────────────┐
│ is_active=true  │  LiveSessionState updated
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────┐
│ create_player_window            │  Spawns Window + PlayerWindow
│ (resource_changed condition)    │
└────────────────┬────────────────┘
                 │
                 v
┌─────────────────────────────────┐
│ setup_player_camera             │  Spawns Camera2d + PlayerCamera
│ (Added<Window> filter)          │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│ sync_player_camera              │  Updates camera transform/projection
│ (runs every frame)              │  to match viewport settings
└────────────────┬────────────────┘
                 │
                 ▼
End Session (ESC or primary window close)
     │
     v
┌─────────────────┐
│ is_active=false │
└────────┬────────┘
         │
         v
┌─────────────────────────────────┐
│ Despawn PlayerWindow +          │
│ PlayerCamera entities           │
└─────────────────────────────────┘
```

## Viewport Indicator

The viewport indicator is an orange rectangle showing what players see:

```
         ┌────────────────────────────┐
         │    Move Handle (tab)       │
         └────────────────────────────┘
                      │
┌─────────────────────┴─────────────────────┐
│  NW ────────── N ────────── NE            │
│   │                          │            │
│   │                          │            │
│   W          center          E            │
│   │            ↑             │            │
│   │        (rotation)        │            │
│  SW ────────── S ────────── SE            │
└───────────────────────────────────────────┘

Handles:
- Corner (NE, NW, SE, SW): Resize maintaining aspect ratio
- Edge (N, S, E, W): Resize maintaining aspect ratio
- Move handle (tab): Reposition viewport
- Arrow at center: Shows "up" direction on player display
```

### Handle Hit Detection

```rust
pub fn get_handle_at_position(
    world_pos: Vec2,
    session_state: &LiveSessionState,
    camera_scale: f32,
) -> ViewportDragMode {
    // Priority: Move handle > Corners > Edges > None
    // Hit areas scale with camera zoom for consistent interaction
}
```

## Camera Synchronization

The player camera is kept in sync with viewport settings:

```rust
pub fn sync_player_camera(
    session_state: Res<LiveSessionState>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<PlayerCamera>>,
) {
    // Position: viewport_center
    transform.translation.x = session_state.viewport_center.x;
    transform.translation.y = session_state.viewport_center.y;

    // Rotation: negative (camera rotates opposite to content)
    transform.rotation = Quat::from_rotation_z(-session_state.rotation_radians());

    // Projection: effective_viewport_size (swaps w/h for portrait)
    ortho.scaling_mode = ScalingMode::Fixed {
        width: effective_size.x,
        height: effective_size.y
    };
}
```

## Graceful Shutdown

Two systems handle window closure:

1. **handle_player_window_close**: ESC in player window ends session
2. **handle_graceful_shutdown**: Primary window close cleans up player window

```rust
pub fn handle_graceful_shutdown(
    mut close_events: MessageReader<WindowCloseRequested>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    player_windows: Query<Entity, With<PlayerWindow>>,
) {
    // If primary window closing, despawn player window first
    // to prevent orphaned window issues
}
```

## Code Example: Starting a Session

```rust
// In layers_panel.rs or session_controls.rs
fn start_session(
    mut session_state: ResMut<LiveSessionState>,
    monitor_info: MonitorInfo,
    initial_viewport: Rect,
) {
    session_state.is_active = true;
    session_state.selected_monitor = Some(monitor_info);
    session_state.viewport_center = initial_viewport.center();
    session_state.viewport_size = initial_viewport.size();
    session_state.rotation_degrees = 0;
    // Window creation happens automatically via resource_changed trigger
}
```

## See Also

- [editor/README.md](../editor/README.md) - session_is_active condition
- [ui/README.md](../ui/README.md) - Monitor selection dialog, session controls
- [map/README.md](../map/README.md) - FogOfWar rendering on player view
