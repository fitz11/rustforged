# Update Module

GitHub Releases API integration for checking application updates.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | UpdateCheckerPlugin, GitHub API client, update UI |

## Key Types

### UpdateState (Resource)

```rust
pub struct UpdateState {
    pub is_checking: bool,        // Currently fetching from GitHub
    pub update_available: bool,   // Newer version exists
    pub latest_version: Option<String>,
    pub release_url: Option<String>,
    pub release_notes: Option<String>,
    pub error: Option<String>,
    pub show_dialog: bool,        // Update details dialog open
    pub dismissed: bool,          // User dismissed for this session
}
```

### GitHubRelease

Response structure from GitHub Releases API:

```rust
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,    // e.g., "v1.2.3"
    html_url: String,    // Release page URL
    name: Option<String>,
    body: Option<String>,  // Release notes (markdown)
    prerelease: bool,
    draft: bool,
}
```

### UpdateCheckTask (Component)

Async task for background update checking:

```rust
#[derive(Component)]
struct UpdateCheckTask(Task<UpdateCheckResult>);
```

## Update Check Flow

```
App Startup
     │
     v
┌─────────────────┐
│start_update_check│  Spawn async task
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│ AsyncComputeTaskPool                     │
│ ┌─────────────────────────────────────┐ │
│ │ check_github_releases()             │ │
│ │ GET /repos/{owner}/{repo}/releases  │ │
│ │     /latest                         │ │
│ └─────────────────────────────────────┘ │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌─────────────────┐
│poll_update_check│  Check if task complete
└────────┬────────┘
         │ Task complete
         ▼
┌─────────────────┐
│ UpdateState     │  Update resource
│ updated         │
└────────┬────────┘
         │
    ┌────┴────┐
    │ Update  │
    │ avail?  │
    └────┬────┘
     Yes │ No
    ┌────┴────────────────┐
    v                     v
┌─────────────┐      (no UI shown)
│ Orange      │
│ indicator   │
│ in toolbar  │
└──────┬──────┘
       │ User clicks
       ▼
┌─────────────┐
│ Update      │
│ Dialog      │
│ [Download]  │
│ [Later]     │
│ [Dismiss]   │
└─────────────┘
```

## Version Comparison

Uses the `semver` crate for proper semantic version comparison:

```rust
let update_available = match (
    Version::parse(version_str),   // From GitHub (stripped 'v' prefix)
    Version::parse(CURRENT_VERSION), // From Cargo.toml
) {
    (Ok(latest), Ok(current)) => latest > current,
    _ => false,  // If parsing fails, assume no update
};
```

## GitHub API Integration

```rust
const GITHUB_REPO: &str = "fitz11/rustforged";

fn check_github_releases() -> UpdateCheckResult {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = ureq::get(&url)
        .set("User-Agent", "rustforged-update-checker")
        .set("Accept", "application/vnd.github.v3+json")
        .call();

    // Handle response, parse JSON, compare versions
}
```

The system:
- Uses `ureq` for synchronous HTTP (run in async task pool)
- Sets proper User-Agent (required by GitHub API)
- Skips draft and prerelease versions
- Handles 404 gracefully (no releases yet)

## UI Components

### Update Indicator

Small orange label in toolbar area when update is available:

```rust
pub fn update_indicator_ui(
    mut contexts: EguiContexts,
    mut update_state: ResMut<UpdateState>,
) -> Result {
    if !update_state.update_available || update_state.dismissed {
        return Ok(());
    }

    egui::TopBottomPanel::top("update_indicator")
        .show(ctx, |ui| {
            ui.colored_label(
                egui::Color32::from_rgb(255, 165, 0),  // Orange
                format!("Update v{} available", version),
            );
        });

    Ok(())
}
```

### Update Dialog

Modal window with version info, release notes, and actions:

```
┌─────────────────────────────────────┐
│ Update Available                [X] │
├─────────────────────────────────────┤
│ Current version: 0.1.0              │
│ Latest version:  0.2.0              │
│                                     │
│ Release notes:                      │
│ ┌─────────────────────────────────┐ │
│ │ - Added new feature X           │ │
│ │ - Fixed bug Y                   │ │
│ │ - Improved performance          │ │
│ └─────────────────────────────────┘ │
│                                     │
│ [Download]  [Later]  [Dismiss]      │
└─────────────────────────────────────┘
```

Actions:
- **Download**: Opens release page in browser via `open::that(url)`
- **Later**: Closes dialog but shows indicator again next startup
- **Dismiss**: Hides indicator for this session

## Constants

```rust
/// Current version from Cargo.toml (compile-time)
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for release checking
const GITHUB_REPO: &str = "fitz11/rustforged";
```

## Error Handling

The update checker handles failures gracefully:

```rust
match response {
    Ok(resp) => {
        // Parse and compare versions
    }
    Err(ureq::Error::Status(404, _)) => {
        // No releases yet - not an error
        UpdateCheckResult { update_available: false, ... }
    }
    Err(e) => {
        // Network/API error
        UpdateCheckResult {
            error: Some(format!("Failed to check: {}", e)),
            ...
        }
    }
}
```

Errors are stored in `UpdateState::error` but not shown to users (updates are optional).

## Plugin Registration

```rust
impl Plugin for UpdateCheckerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UpdateState>()
            .add_systems(Startup, start_update_check)
            .add_systems(Update, poll_update_check)
            .add_systems(
                EguiPrimaryContextPass,
                (update_indicator_ui, update_dialog_ui),
            );
    }
}
```

## Code Example: Customizing Update Check

To modify the update check behavior:

```rust
// Change repository
const GITHUB_REPO: &str = "your-org/your-repo";

// Add custom headers
ureq::get(&url)
    .set("User-Agent", "your-app-name")
    .set("Authorization", format!("token {}", token))  // For private repos
    .call();

// Check for prereleases
if !release.draft {  // Allow prereleases
    // Compare versions...
}
```

## Dependencies

- `ureq` - HTTP client
- `semver` - Version parsing and comparison
- `serde` - JSON deserialization
- `futures_lite` - Async polling
- `open` - Open URLs in browser

## See Also

- [ui/README.md](../ui/README.md) - Dialog patterns
- [config/README.md](../config/README.md) - Version stored in config (future)
