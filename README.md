# Rustforged

<p align="center">
  <img src="packaging/icons/Rustforged.png" alt="Rustforged" width="128">
</p>

A D&D 5E Virtual Tabletop (VTT) map editor built with Bevy 0.18 and bevy_egui 0.39.

## Features

- **Layer-based map editing** - Background, Terrain, Doodad, Token, GM, Annotation, and Fog of War layers
- **Asset library management** - Create, open, and organize custom asset libraries
- **Drawing tools** - Freehand drawing and straight line annotations
- **Undo/redo** - Full command history for placement, movement, deletion, and annotations
- **Fog of War** - Reveal/hide map areas for players
- **Grid system** - 70px grid with snap-to-grid placement (hold Shift for free placement)
- **Live session mode** - Display player view on a secondary monitor with configurable viewport
- **Map persistence** - Save and load maps as JSON files

## Downloads

Pre-built installers are available for Windows and macOS:

| Platform | Download |
|----------|----------|
| Windows (x64) | [MSI Installer](https://github.com/fitz11/rustforged/releases/latest) |
| Windows (ARM64) | [MSI Installer](https://github.com/fitz11/rustforged/releases/latest) |
| macOS (Apple Silicon) | [DMG](https://github.com/fitz11/rustforged/releases/latest) |
| Linux (x86_64) | [Tarball](https://github.com/fitz11/rustforged/releases/latest) (`.tar.gz`) — or build from source (recommended, see below) |

## Quick Start

### Build from Source

```bash
# Install Rust via rustup.rs, then:
git clone https://github.com/fitz11/rustforged.git
cd rustforged
cargo run
```

**Linux dependencies:** `libasound2-dev libudev-dev pkg-config` (Debian/Ubuntu)

### Linux Tarball

The Linux release is a plain `.tar.gz` (built for **x86_64**, no installer). Building from source is recommended for the simplest experience — the tarball is just an unpacked binary with no desktop integration or dependency management, and it's forward-compatible only from the glibc it was built against (Ubuntu LTS).

```bash
# Extract, then run the binary directly:
tar -xzf rustforged-*-linux-x86_64.tar.gz
cd rustforged-linux
chmod +x rustforged      # ensure the executable bit survived extraction
./rustforged
```

The archive contains the `rustforged` binary plus the default `assets/` library — run it from inside `rustforged-linux/` so it finds them.

If the binary fails to start with a missing-library error (`error while loading shared libraries: ...`), install the runtime libraries for your distro:

- **Ubuntu / Debian:** `sudo apt install libasound2 libudev1 libxkbcommon0 libwayland-client0`
- **Arch:** `sudo pacman -S alsa-lib systemd-libs libxkbcommon wayland`

(Most desktop installs already have these.) If you hit a glibc version mismatch on a rolling distro like Arch, build from source instead.

## Controls

### Camera

| Action | Control |
|--------|---------|
| Pan | Middle-mouse drag |
| Zoom | Scroll wheel |

### Tool Shortcuts

| Key | Tool |
|-----|------|
| V / S | Select - Click to select, drag to move |
| P | Place - Single-click asset placement |
| B | Brush - Continuous placement while dragging |
| D | Draw - Freehand annotation paths |
| L | Line - Straight line annotations |
| F | Fog - Reveal/hide fog of war areas |
| C / Shift+C | Cycle layer (Place/Brush tools) |

### Selection & Editing

| Action | Control |
|--------|---------|
| Select item | Click |
| Multi-select | Ctrl+Click or box select |
| Move selected | Drag |
| Resize selected | Drag handles |
| Fit to grid | G |
| Rotate 90° | R / Shift+R |
| Restore aspect ratio | A |
| Delete | Delete or Backspace |
| Copy/Cut/Paste | Ctrl+C / Ctrl+X / Ctrl+V |
| Undo/Redo | Ctrl+Z / Ctrl+Y (or Ctrl+Shift+Z) |

### File Operations

| Action | Shortcut |
|--------|----------|
| Save | Ctrl+S |
| Save as | Ctrl+Shift+S |
| Open | Ctrl+O |
| New | Ctrl+N |
| Help | H |

## Asset Library

Assets are loaded from `assets/library/` by default with subdirectories: `unsorted/`, `terrain/`, `doodads/`, `tokens/`.

You can open or create custom asset libraries via the left panel. Maps are saved to `<library>/maps/`.

**Supported formats:** PNG, JPG, JPEG, WebP, GIF, BMP, TIFF

## Live Session Mode

1. Click "Start Session" in the toolbar
2. Select which monitor to display the player view
3. Drag the viewport indicator to frame the player view
4. Resize using corner handles (maintains aspect ratio)

The player window shows a fullscreen view without annotations or viewport indicator.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| bevy | 0.18 | Game engine (rendering, ECS, windowing) |
| bevy_egui | 0.39 | Immediate-mode UI integration |
| image | 0.25 | Image metadata reading |
| serde / serde_json | 1.x | JSON serialization |
| rfd | 0.15 | Native file dialogs |
| ureq | 2.x | HTTP client for update checking |
| semver | 1.x | Version parsing |
| open | 5.x | Open URLs in browser |
| futures-lite | 2.x | Async utilities |
| zip | 7.x | Archive extraction for updates |
| keepawake | 0.6 | Prevent display sleep during live sessions |
| tracing-subscriber | 0.3 | Logging framework |
| tracing-appender | 0.2 | Log file output |
| chrono | 0.4 | Timestamps in logs |
| dirs | 6.x | Platform-specific directories |

## License

MIT OR Apache-2.0
