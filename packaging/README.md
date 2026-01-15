# Packaging & Installers

This directory contains configuration and assets for building platform-specific installers.

## Overview

Rustforged uses [cargo-packager](https://github.com/crabnebula-dev/cargo-packager) to create installers:

| Platform | Format | Output |
|----------|--------|--------|
| Windows x64 | MSI | `target/release/msi/*.msi` |
| Windows ARM64 | MSI | `target/aarch64-pc-windows-msvc/release/msi/*.msi` |
| macOS ARM64 | DMG | `target/aarch64-apple-darwin/release/dmg/*.dmg` |
| Linux x64 | tar.gz | Built manually (no installer) |

## Directory Structure

```
packaging/
├── README.md           # This file
└── icons/
    ├── README.md       # Icon generation instructions
    ├── icon.png        # Source icon (you provide this)
    ├── icon.ico        # Windows icon (generated)
    └── icon.icns       # macOS icon (generated)
```

## Prerequisites

### Install cargo-packager

```bash
cargo install cargo-packager --locked
```

### Application Icons

Before building installers, you need icons in `packaging/icons/`:

1. Create or obtain a source `icon.png` (256x256 minimum, 512x512+ recommended)
2. Generate platform-specific icons - see `packaging/icons/README.md`

## Building Installers Locally

### Windows (x64)

Run on a Windows machine:

```bash
cargo packager --release --formats msi
```

Output: `target/release/msi/Rustforged_<version>_x64_en-US.msi`

### Windows (ARM64)

Cross-compile on Windows (requires ARM64 target):

```bash
rustup target add aarch64-pc-windows-msvc
cargo packager --release --target aarch64-pc-windows-msvc --formats msi
```

Output: `target/aarch64-pc-windows-msvc/release/msi/Rustforged_<version>_arm64_en-US.msi`

### macOS (Apple Silicon)

Run on a Mac with Apple Silicon:

```bash
rustup target add aarch64-apple-darwin
cargo packager --release --target aarch64-apple-darwin --formats dmg
```

Output: `target/aarch64-apple-darwin/release/dmg/Rustforged_<version>_aarch64.dmg`

### Linux

Linux users build from source. For distribution, create a tar.gz manually:

```bash
cargo build --release
mkdir -p dist/rustforged
cp target/release/rustforged dist/rustforged/
cp -r assets dist/rustforged/
cp README.md dist/rustforged/
tar -czvf rustforged-linux-x86_64.tar.gz -C dist rustforged
```

## How It Works

### Configuration

The `packager.toml` file in the repository root defines:

- **Package metadata**: Name, version, description, license
- **Icons**: Platform-specific icon paths
- **Resources**: Files bundled with the application (the `assets/` directory)
- **Platform settings**: OS-specific options (installer type, macOS category, etc.)

### What Gets Bundled

Each installer includes:

1. **The executable**: `rustforged` (or `rustforged.exe` on Windows)
2. **Assets directory**: Contains fonts, default library, and maps
3. **Application icon**: Displayed in file explorer, dock, taskbar

### Installation Locations

| Platform | Default Install Path |
|----------|---------------------|
| Windows | `C:\Program Files\Rustforged\` |
| macOS | `/Applications/Rustforged.app` |
| Linux | User extracts to preferred location |

### Windows MSI Details

The MSI installer:
- Adds Start Menu shortcut
- Registers uninstaller in Add/Remove Programs
- Copies all files to Program Files
- Does NOT require admin rights for per-user install

### macOS DMG Details

The DMG contains:
- `Rustforged.app` bundle (drag to Applications)
- The app bundle contains the executable and all resources
- First launch may show Gatekeeper warning (unsigned)

## Automated Releases

GitHub Actions builds installers automatically:

1. Go to **Actions** > **Release** workflow
2. Click **Run workflow**
3. Enter version number (e.g., `0.1.0`)
4. Wait for all build jobs to complete
5. Find artifacts in the draft release

## Code Signing (Future)

Currently, installers are unsigned. Users may see warnings:

- **Windows**: SmartScreen warning - click "More info" > "Run anyway"
- **macOS**: Gatekeeper blocks - right-click > "Open" > "Open"

To enable code signing later:

### Windows

1. Obtain a code signing certificate (EV or standard)
2. Add certificate to GitHub secrets
3. Configure in `packager.toml`:
   ```toml
   [package.windows]
   certificate = "path/to/cert.pfx"
   certificate_password = "${WINDOWS_CERT_PASSWORD}"
   ```

### macOS

1. Enroll in Apple Developer Program ($99/year)
2. Create signing certificate and notarization credentials
3. Add to GitHub secrets
4. Configure in `packager.toml`:
   ```toml
   [package.macos]
   signing-identity = "Developer ID Application: Your Name (TEAMID)"
   notarization-credentials = { apple-id = "${APPLE_ID}", password = "${APPLE_PASSWORD}", team-id = "TEAMID" }
   ```

## Troubleshooting

### "Icons not found" error

Ensure icons exist at:
- `packaging/icons/icon.ico` (Windows)
- `packaging/icons/icon.icns` (macOS)

### "cargo-packager not found"

Install it:
```bash
cargo install cargo-packager --locked
```

### Windows ARM64 build fails

Ensure the target is installed:
```bash
rustup target add aarch64-pc-windows-msvc
```

### macOS build fails on non-Mac

macOS DMGs can only be built on macOS. Use GitHub Actions or a Mac.

### Assets not included in installer

Check `packager.toml` has:
```toml
[package.resources]
"assets" = "assets"
```
