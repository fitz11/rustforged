# Packaging & Installers

This directory contains configuration and assets for building platform-specific installers.

## Overview

Rustforged uses [cargo-packager](https://github.com/crabnebula-dev/cargo-packager) to create installers:

| Platform | Format | Output |
|----------|--------|--------|
| Windows x64 | MSI | `target/release/packager/*.msi` |
| Windows ARM64 | MSI | `target/release/packager/*.msi` |
| macOS ARM64 | DMG | `target/release/packager/*.dmg` |
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
cargo build --release
cargo packager --release --formats wix
```

Output: `target/release/packager/Rustforged_<version>_x64_en-US.msi`

### Windows (ARM64)

Cross-compile on Windows (requires ARM64 target):

```bash
rustup target add aarch64-pc-windows-msvc
cargo build --release --target aarch64-pc-windows-msvc
cargo packager --release --target aarch64-pc-windows-msvc --formats wix
```

Output: `target/release/packager/Rustforged_<version>_arm64_en-US.msi`

### macOS (Apple Silicon)

Run on a Mac with Apple Silicon:

```bash
cargo build --release
cargo packager --release --formats dmg
```

Output: `target/release/packager/Rustforged_<version>_aarch64.dmg`

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

The `packager.toml` file in the repository root uses a flat structure (no `[package]` wrapper):

```toml
# Top-level fields
name = "rustforged"
product-name = "Rustforged"
version = "0.1.0"
identifier = "com.fitz11.rustforged"
out-dir = "target/release/packager"
binaries-dir = "target/release"

# Icons as a flat array (not per-platform)
icons = ["packaging/icons/icon.ico", "packaging/icons/icon.icns"]

# Resources as array of {src, target} objects
resources = [{ src = "assets", target = "assets" }]

# Binaries section
[[binaries]]
path = "rustforged"
main = true

# Platform-specific settings
[macos]
minimum-system-version = "11.0"
```

Key format notes:
- All fields are at the top level (no `[package]` section)
- `icons` is a flat array, not a table with `windows`/`macos` keys
- `resources` is an array of objects with `src` and `target` fields
- `binaries-dir` must point to where `cargo build --release` outputs the binary

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
   [windows]
   certificate-thumbprint = "YOUR_CERT_THUMBPRINT"
   ```

### macOS

1. Enroll in Apple Developer Program ($99/year)
2. Create signing certificate and notarization credentials
3. Add to GitHub secrets
4. Configure in `packager.toml`:
   ```toml
   [macos]
   signing-identity = "Developer ID Application: Your Name (TEAMID)"
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

### "Couldn't detect a valid configuration file" error

This usually means the `packager.toml` format is invalid. Common issues:
- Using `[package]` wrapper (fields should be at top level)
- Using `[icons]` table instead of `icons = [...]` array
- Placing fields after `[[binaries]]` section (they become part of that table)

Run with verbose mode to see the actual parse error:
```bash
cargo packager --release -v
```

### Assets not included in installer

Check `packager.toml` has resources defined as an array (at the top level, before `[[binaries]]`):
```toml
resources = [{ src = "assets", target = "assets" }]
```
