# Distribution Configuration

This directory contains platform-specific configuration files for packaging and distribution.

## Directory Structure

```
dist/
├── windows/     # Windows installer configuration (WiX MSI)
├── macos/       # macOS app bundle configuration
│   └── Info.plist
└── linux/       # Linux desktop integration
    └── rustforged.desktop
```

## macOS

- `Info.plist` - App bundle metadata for creating a proper .app bundle
- When code signing is set up, add signing configuration here

## Linux

- `rustforged.desktop` - Desktop entry file for application launchers
- Used when creating AppImage, Flatpak, or system packages

## Windows

- Future: WiX MSI installer configuration
- Future: NSIS installer script (alternative)

## Building Releases

Release builds are automated via GitHub Actions. See `.github/workflows/release.yml`.

To build a release locally:

```bash
# Build without dynamic linking (required for distribution)
cargo build --release --no-default-features
```
