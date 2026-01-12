# UI Module

egui-based user interface panels, dialogs, and controls.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | UiPlugin, DialogState resource, system scheduling |
| `toolbar.rs` | Top toolbar (tools, colors, grid settings) |
| `asset_browser.rs` | Left panel (file menu, library browser, asset grid) |
| `layers_panel.rs` | Right panel (layers, properties, session controls) |
| `file_menu.rs` | FileMenuState, confirmation dialogs, error dialogs |
| `asset_import.rs` | Drag-drop import workflow |
| `session_controls.rs` | Monitor selection dialog |
| `settings_dialog.rs` | Application preferences |

## Key Types

### DialogState (Resource)

Tracks whether any modal dialog is open:

```rust
#[derive(Resource, Default)]
pub struct DialogState {
    pub any_modal_open: bool,
}
```

This is used by `no_dialog_open` run condition in the editor module to prevent editor input while dialogs are open.

### AssetBrowserState (Resource)

```rust
pub struct AssetBrowserState {
    pub current_folder: PathBuf,
    pub search_text: String,
    pub rename_dialog_open: bool,
    pub rename_map_dialog_open: bool,
    pub rename_library_dialog_open: bool,
    pub show_set_default_dialog: bool,
    // ... thumbnails, scroll state
}
```

### FileMenuState (Resource)

```rust
pub struct FileMenuState {
    pub show_new_confirmation: bool,
    pub show_save_name_dialog: bool,
    pub save_name: String,
    // ...
}
```

### AssetImportDialog (Resource)

```rust
pub struct AssetImportDialog {
    pub is_open: bool,
    pub source_path: Option<PathBuf>,
    pub target_category: AssetCategory,
    pub new_filename: String,
}
```

## Panel Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  Toolbar (top)                                                      │
│  [Select] [Place] [Brush] [Draw] [Line] [Text] [Fog]  Grid: [70]   │
├──────────────────┬────────────────────────────┬─────────────────────┤
│                  │                            │                     │
│  Asset Browser   │      Map Canvas            │  Layers Panel       │
│  (left)          │      (center)              │  (right)            │
│                  │                            │                     │
│  ┌────────────┐  │                            │  ┌───────────────┐  │
│  │ File Menu  │  │                            │  │ Layer List    │  │
│  │ > New      │  │                            │  │ [x] Background│  │
│  │ > Open     │  │                            │  │ [x] Terrain   │  │
│  │ > Save     │  │                            │  │ [x] Doodad    │  │
│  └────────────┘  │                            │  │ [x] Token     │  │
│                  │                            │  └───────────────┘  │
│  ┌────────────┐  │                            │                     │
│  │ Categories │  │                            │  ┌───────────────┐  │
│  │ > Unsorted │  │                            │  │ Properties    │  │
│  │ > Terrain  │  │                            │  │ Position: ... │  │
│  │ > Doodads  │  │                            │  │ Rotation: ... │  │
│  │ > Tokens   │  │                            │  └───────────────┘  │
│  └────────────┘  │                            │                     │
│                  │                            │  ┌───────────────┐  │
│  ┌────────────┐  │                            │  │ Session       │  │
│  │ Asset Grid │  │                            │  │ [Start]       │  │
│  │ [img][img] │  │                            │  │ Rotate: CW CCW│  │
│  │ [img][img] │  │                            │  └───────────────┘  │
│  └────────────┘  │                            │                     │
│                  │                            │                     │
└──────────────────┴────────────────────────────┴─────────────────────┘
```

## bevy_egui Integration Pattern

All UI systems must follow this pattern:

```rust
pub fn my_ui_system(mut contexts: EguiContexts) -> Result {
    let ctx = contexts.ctx_mut()?;

    egui::Window::new("Title").show(ctx, |ui| {
        // UI code
    });

    Ok(())
}
```

The `Result` return type and `?` on `ctx_mut()` handle the case where the egui context isn't available (e.g., during startup).

## System Scheduling

UI systems run in the `EguiPrimaryContextPass` schedule with specific ordering:

```rust
// UiPlugin::build()
app.add_systems(
    EguiPrimaryContextPass,
    (
        // First: side panels (reserve space)
        asset_browser::asset_browser_ui,
        layers_panel::layers_panel_ui,
    ).chain(),
)
.add_systems(
    EguiPrimaryContextPass,
    (
        // Second: top panels (fit between side panels)
        toolbar::toolbar_ui,
        toolbar::tool_settings_ui,
    ).chain()
     .after(asset_browser::asset_browser_ui)
     .after(layers_panel::layers_panel_ui),
)
.add_systems(
    EguiPrimaryContextPass,
    (
        // Last: dialogs/overlays (on top of everything)
        file_menu::file_menu_ui,
        asset_import::asset_import_ui,
        settings_dialog::settings_dialog_ui,
        // ...other dialogs
    ).after(toolbar::toolbar_ui),
)
```

## Dialog State Aggregation

The `update_dialog_state` system runs in `First` schedule to aggregate all dialog flags:

```rust
fn update_dialog_state(
    file_menu: Res<file_menu::FileMenuState>,
    asset_browser: Res<asset_browser::AssetBrowserState>,
    asset_import: Res<asset_import::AssetImportDialog>,
    settings: Res<settings_dialog::SettingsDialogState>,
    // ... 10+ more resources
    mut dialog_state: ResMut<DialogState>,
) {
    dialog_state.any_modal_open =
        file_menu.show_new_confirmation
        || file_menu.show_save_name_dialog
        || asset_browser.rename_dialog_open
        || asset_import.is_open
        || settings.is_open
        // ... all dialog flags
        || async_op.is_busy();
}
```

## Dialogs Overview

### Modal Dialogs

| Dialog | Resource | Trigger |
|--------|----------|---------|
| New Map Confirmation | `FileMenuState::show_new_confirmation` | File > New (with unsaved changes) |
| Save As | `FileMenuState::show_save_name_dialog` | File > Save As |
| Rename Asset | `AssetBrowserState::rename_dialog_open` | Right-click asset > Rename (F2) |
| Move Asset | `AssetBrowserState::move_dialog_open` | Move button in Selected Asset section |
| Rename Map | `AssetBrowserState::rename_map_dialog_open` | Right-click map > Rename (F3) |
| Rename Library | `AssetBrowserState::rename_library_dialog_open` | Right-click library > Rename (F4) |
| Set Default Library | `AssetBrowserState::show_set_default_dialog` | File > Set as Default |
| Asset Import | `AssetImportDialog::is_open` | Drag file into window |
| Settings | `SettingsDialogState::is_open` | File > Settings |
| Monitor Selection | `MonitorSelectionDialog::is_open` | Start Session button |
| Help | `HelpWindowState::is_open` | Help menu or H key |

### Error/Warning Dialogs

| Dialog | Resource | Trigger |
|--------|----------|---------|
| Unsaved Changes | `UnsavedChangesDialog` | Close window with unsaved work |
| Missing Map | `MissingMapWarning` | Last opened map not found |
| Save Error | `MapSaveError` | File write failed |
| Load Error | `MapLoadError` | File read/parse failed |
| Save Validation | `SaveValidationWarning` | Map references missing assets |
| Load Validation | `LoadValidationWarning` | Loaded map has missing assets |
| Config Reset | `ConfigResetNotification` | Config file was corrupted |

## Thumbnail System

Asset thumbnails are loaded lazily to avoid UI hitches:

```rust
// In asset_browser.rs
pub fn load_and_register_thumbnails(
    mut state: ResMut<AssetBrowserState>,
    mut contexts: EguiContexts,
) {
    // Process a few thumbnails per frame
    const MAX_PER_FRAME: usize = 3;

    for _ in 0..MAX_PER_FRAME {
        if let Some(asset) = state.pending_thumbnails.pop() {
            // Load image, create egui texture, register in cache
            let texture_id = ctx.load_texture(...);
            state.thumbnail_cache.insert(asset.path, texture_id);
        }
    }
}
```

## UI Pointer Check

Input systems should check if the cursor is over UI before processing:

```rust
pub fn handle_placement(
    mut contexts: EguiContexts,
    // ...
) {
    if let Ok(ctx) = contexts.ctx_mut()
        && ctx.is_pointer_over_area()
    {
        return;  // Don't place if clicking on UI
    }

    // Process placement...
}
```

## Code Example: Adding a New Dialog

1. Add state to appropriate resource or create new one:

```rust
// In new_feature.rs or existing file
#[derive(Resource, Default)]
pub struct NewFeatureDialog {
    pub is_open: bool,
    pub some_value: String,
}
```

2. Create UI system:

```rust
pub fn new_feature_dialog_ui(
    mut contexts: EguiContexts,
    mut dialog: ResMut<NewFeatureDialog>,
) -> Result {
    if !dialog.is_open {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    let mut open = true;
    egui::Window::new("New Feature")
        .open(&mut open)
        .show(ctx, |ui| {
            // Dialog content
            if ui.button("Confirm").clicked() {
                // Handle action
                dialog.is_open = false;
            }
        });

    if !open {
        dialog.is_open = false;
    }

    Ok(())
}
```

3. Register in `UiPlugin`:

```rust
// In mod.rs
.init_resource::<NewFeatureDialog>()
.add_systems(EguiPrimaryContextPass, new_feature_dialog_ui)
```

4. Add to dialog state aggregation:

```rust
// In update_dialog_state()
dialog_state.any_modal_open =
    // ... existing flags
    || new_feature_dialog.is_open;
```

## See Also

- [editor/README.md](../editor/README.md) - Tool handling, no_dialog_open condition
- [assets/README.md](../assets/README.md) - AssetLibrary, SelectedAsset
- [session/README.md](../session/README.md) - Monitor selection, session controls
- [config/README.md](../config/README.md) - Settings persistence
