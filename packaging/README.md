# Packaging & Installers

This directory contains configuration and assets for building platform-specific installers.

## Overview

Rustforged uses [cargo-packager](https://github.com/crabnebula-dev/cargo-packager) to create installers:

| Platform | Format | Output |
|----------|--------|--------|
| Windows x64 | MSI | `target/release/packager/Rustforged_<version>_x64_en-US.msi` |
| Windows ARM64 | MSI | `target/release/packager/Rustforged_<version>_arm64_en-US.msi` |
| macOS Apple Silicon | DMG | `target/release/packager/Rustforged_<version>_aarch64.dmg` |
| macOS Intel | DMG | `target/release/packager/Rustforged_<version>_x64.dmg` |
| Linux x64 | tar.gz | Built manually (no installer) |

> **Configuration lives in `Cargo.toml`.** All packaging settings are under
> `[package.metadata.packager]` (and `[package.metadata.packager.macos]`) in the
> repository-root `Cargo.toml` — there is **no** separate `packager.toml`. The version,
> description, and authors are inherited from the `[package]` section, so they are never
> duplicated.

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

### macOS (both architectures)

Run on any Mac. The `scripts/build-macos.sh` helper builds **both** Apple Silicon and
Intel DMGs in one go (either arch can be built from either kind of Mac):

```bash
./scripts/build-macos.sh
```

To build a single architecture by hand:

```bash
# Apple Silicon
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo packager --release --target aarch64-apple-darwin \
  --binaries-dir target/aarch64-apple-darwin/release --formats dmg

# Intel
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo packager --release --target x86_64-apple-darwin \
  --binaries-dir target/x86_64-apple-darwin/release --formats dmg
```

Output:
- `target/release/packager/Rustforged_<version>_aarch64.dmg` (Apple Silicon)
- `target/release/packager/Rustforged_<version>_x64.dmg` (Intel)

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

Packaging is configured in the repository-root **`Cargo.toml`** under the
`[package.metadata.packager]` tables (cargo-packager reads Cargo metadata directly —
there is no standalone `packager.toml`):

```toml
[package]
name = "rustforged"
version = "0.1.2"                       # single source of truth for the release version
description = "D&D 5E Virtual Tabletop map editor"

[package.metadata.packager]
# `version`, `description`, and `authors` are inherited from [package] when omitted here,
# so the version is never duplicated.
product-name = "Rustforged"
identifier = "dev.squishygoose.rustforged"
publisher = "squishygoose"
copyright = "Copyright © 2026 Rustforged Contributors"
icons = ["packaging/icons/icon.ico", "packaging/icons/icon.icns"]
out-dir = "target/release/packager"

[package.metadata.packager.macos]
minimum-system-version = "10.15"        # Intel-inclusive floor; keeps LSMinimumSystemVersion correct
```

Key notes:
- Fields use camelCase or kebab-case (both accepted, e.g. `product-name` / `productName`).
- `version` is intentionally omitted from `[package.metadata.packager]` so it tracks
  `[package].version` — do not add it back.
- The macOS `Info.plist` is **generated** from this config. cargo-packager already sets
  `NSHighResolutionCapable`, `CFBundleShortVersionString` (from the version),
  `CFBundleIdentifier`, and `NSHumanReadableCopyright` — there is no hand-written plist.

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
3. Configure in `Cargo.toml`:
   ```toml
   [package.metadata.packager.windows]
   certificate-thumbprint = "YOUR_CERT_THUMBPRINT"
   ```

### macOS

1. Enroll in Apple Developer Program ($99/year)
2. Create a Developer ID Application certificate and notarization credentials
3. Add them to GitHub secrets
4. Configure in `Cargo.toml` (cargo-packager supports signing and notarization natively):
   ```toml
   [package.metadata.packager.macos]
   signing-identity = "Developer ID Application: Your Name (TEAMID)"
   # notarization credentials are supplied via env vars / config; see cargo-packager docs
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

cargo-packager reads its config from `[package.metadata.packager]` in `Cargo.toml`. If it
can't find the config, check that:
- You are running `cargo packager` from the crate root (where `Cargo.toml` lives).
- The `[package.metadata.packager]` table is present and uses valid keys (unknown keys are
  rejected — `deny_unknown_fields`).

Run with verbose mode to see the actual parse error:
```bash
cargo packager --release -v
```

### macOS build only produces one architecture

Pass an explicit `--target`. Building both DMGs requires two runs (one per target), which
is exactly what `scripts/build-macos.sh` and the CI `build-macos-*` jobs do.
