# Layers Panel Module

Right-side panel UI for layers, fog of war, properties, and live session controls.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports, `HelpWindowState` resource |
| `layers.rs` | Layer visibility/lock controls UI |
| `fog.rs` | Fog of War toggle and reset controls |
| `properties.rs` | Selected item properties editor (position, rotation, scale, layer) |
| `session.rs` | Live Session viewport controls (position, size, rotation) |
| `main_panel.rs` | Main panel orchestration, `layers_panel_ui` system |
| `help.rs` | Help popup window, `handle_help_shortcut` system |

## Key Types

- **HelpWindowState**: Resource tracking whether the help window is open

## Systems

- **layers_panel_ui**: Main layers panel rendering system
- **help_popup_ui**: Help popup window rendering system
- **handle_help_shortcut**: Keyboard handler for H key (toggle help)

## Panel Sections

1. **Layers**: Toggle visibility and lock state for each layer
2. **Fog of War**: Enable/disable fog, reset revealed areas
3. **Properties**: Edit selected item(s) - position, rotation, scale, layer, z-index
4. **Live Session**: Viewport controls when session is active
5. **Help Button**: Opens keyboard shortcuts reference
