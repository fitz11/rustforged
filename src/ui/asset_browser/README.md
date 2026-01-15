# Asset Browser Module

Left-side panel UI for managing asset libraries, maps, and browsing assets.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports, documentation |
| `state.rs` | `AssetBrowserState` resource, `MapResources`, `DialogStates` SystemParams |
| `helpers.rs` | Utility functions: `scan_maps_directory`, `discover_folders`, `extension_color` |
| `library_ops.rs` | Library export/import: `export_library_to_zip`, `import_library_from_zip` |
| `asset_ops.rs` | Asset file operations: `rename_asset`, `move_asset`, `update_asset_paths_in_maps` |
| `thumbnails.rs` | Thumbnail loading system: `load_and_register_thumbnails` |
| `dialogs.rs` | Dialog windows: rename asset/map/library, move asset, import error, success |
| `main_panel.rs` | Main panel UI: `asset_browser_ui` system |

## Key Types

- **AssetBrowserState**: Resource tracking browser panel state (selected folder, cached maps, dialog state)
- **MapResources**: SystemParam bundling map-related resources and event writers
- **DialogStates**: SystemParam bundling dialog state resources

## Systems

- **asset_browser_ui**: Main asset browser panel rendering system
- **load_and_register_thumbnails**: Thumbnail loading system (runs before egui pass)

## Panel Sections

1. **Library Header**: Library name, path, expand/collapse toggle
2. **Library Management**: Open, New, Rename, Export, Import buttons
3. **Maps**: Open maps list, saved maps browser, New/Save/Rename buttons
4. **Assets**: Import button, Open Folder button
5. **Folder Tree**: Hierarchical folder browser
6. **Asset List**: Thumbnails and names of assets in selected folder
7. **Selected Asset**: Metadata display with Rename/Move buttons
8. **Settings**: Opens settings dialog
