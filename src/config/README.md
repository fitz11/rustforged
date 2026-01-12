# Config Module

Handles application configuration persistence, including default library paths and recent libraries.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | ConfigPlugin, AppConfig resource, load/save systems |

## Key Types

### AppConfig (Resource)

Runtime configuration resource:

```rust
pub struct AppConfig {
    pub data: AppConfigData,   // Persisted settings
    pub config_path: PathBuf,  // Location of config file
    pub dirty: bool,           // Needs saving
}
```

### AppConfigData

Settings that persist to disk:

```rust
pub struct AppConfigData {
    pub default_library_path: Option<PathBuf>,
    pub recent_libraries: Vec<PathBuf>,
    pub last_map_path: Option<PathBuf>,
}
```

## Config File Location

Platform-specific configuration directory:

| Platform | Path |
|----------|------|
| Linux | `~/.config/rustforged/config.json` |
| macOS | `~/Library/Application Support/rustforged/config.json` |
| Windows | `%APPDATA%\rustforged\config.json` |

## Messages

| Message | Purpose |
|---------|---------|
| `SaveConfigRequest` | Trigger config file write |
| `SetDefaultLibraryRequest` | Set default library path |
| `AddRecentLibraryRequest` | Add to recent libraries list |
| `UpdateLastMapPathRequest` | Remember last opened map |

## UI Resources

Resources for dialog state (shown by UI systems):

```rust
pub struct MissingMapWarning {
    pub show: bool,
    pub path: Option<PathBuf>,
}

pub struct ConfigResetNotification {
    pub show: bool,
    pub reason: Option<String>,
}
```

## Config Load Flow

```
Startup
   │
   v
┌────────────────┐
│ load_config()  │
└───────┬────────┘
        │
   ┌────┴────┐
   │ Exists? │
   └────┬────┘
    Yes │ No
   ┌────┴────────────────┐
   v                     v
┌─────────┐    ┌────────────────┐
│ Parse   │    │ Create default │
│ JSON    │    │ config         │
└────┬────┘    └────────────────┘
     │
┌────┴─────┐
│ Valid?   │
└────┬─────┘
  Yes│ No
┌────┴─────────────────┐
v                      v
AppConfig          ConfigResetNotification
resource           dialog shown
```

## Code Example

### Saving Configuration

```rust
fn save_config_system(
    events: MessageReader<SaveConfigRequest>,
    config: Res<AppConfig>,
) {
    for _ in events.read() {
        let json = serde_json::to_string_pretty(&config.data)?;
        std::fs::write(&config.config_path, json)?;
    }
}
```

## See Also

- [ui/README.md](../ui/README.md) - Settings dialog
- [assets/README.md](../assets/README.md) - Default library initialization
