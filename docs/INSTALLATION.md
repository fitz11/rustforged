# Installation Guide

This guide covers installing Rustforged on all supported platforms.

## Quick Start

| Platform | Method |
|----------|--------|
| Windows (x64/ARM64) | Download and run the MSI installer from [Releases](https://github.com/fitz11/rustforged/releases/latest) |
| macOS (Apple Silicon) | Download the DMG, drag to Applications |
| Linux | Build from source (see below) |

## Windows Installation

### Download

1. Go to [Releases](https://github.com/fitz11/rustforged/releases/latest)
2. Download `Rustforged_<version>_x64_en-US.msi` (or `arm64` for ARM devices)
3. Run the installer

### Installation Location

- Default: `C:\Program Files\Rustforged\`
- User data: `%APPDATA%\Rustforged\` (maps, config, libraries)

### SmartScreen Warning

The installer is currently unsigned. Windows may show a SmartScreen warning:
1. Click "More info"
2. Click "Run anyway"

### Uninstalling

Use "Add or Remove Programs" in Windows Settings.

## macOS Installation

### Download

1. Go to [Releases](https://github.com/fitz11/rustforged/releases/latest)
2. Download `Rustforged_<version>_aarch64.dmg`
3. Open the DMG and drag Rustforged to Applications

### Gatekeeper Warning

The app is currently unsigned. macOS may block it:
1. Right-click the app in Applications
2. Select "Open"
3. Click "Open" in the dialog

### User Data Location

- `~/Library/Application Support/Rustforged/` (maps, config, libraries)

## Linux Installation

Linux users build from source. See the [README](../README.md#installation) for prerequisites and build instructions.

### User Data Location

- `~/.local/share/rustforged/` (maps, config, libraries)

## Auto-Update

Rustforged checks for updates automatically when you start the application.

### How It Works

1. On startup, the app fetches a release manifest from GitHub
2. If a newer version exists, an orange notification appears in the toolbar
3. Click the notification to see release details
4. Choose "Download & Install" for automatic update, or "View Release" to download manually

### Update Behavior by Platform

| Platform | Behavior |
|----------|----------|
| Windows | Downloads MSI, runs installer in passive mode, restarts automatically |
| macOS | Downloads DMG, opens it for you to drag to Applications |
| Linux | Shows release page link (build from source) |

### Disabling Update Checks

Update checks run on startup. You can dismiss the notification for the current session by clicking "Dismiss". There is currently no setting to permanently disable update checks.

## Troubleshooting

### Windows: "Windows protected your PC" (SmartScreen)

This appears because the installer is unsigned. Click "More info" then "Run anyway".

### macOS: "Rustforged can't be opened because it is from an unidentified developer"

Right-click the app, select "Open", then click "Open" in the dialog. You only need to do this once.

### Application won't start

1. Check that your system meets minimum requirements
2. On Linux, ensure required system libraries are installed (see README)
3. Try running from terminal to see error messages

### Update check fails

- Check your internet connection
- The app will continue working; you can update manually from the [Releases](https://github.com/fitz11/rustforged/releases) page

## Building Custom Installers

For developers who want to build their own installers, see [packaging/README.md](../packaging/README.md).
