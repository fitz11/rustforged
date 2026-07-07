# Distribution Configuration

This directory contains platform-specific configuration files for packaging and distribution.

## Directory Structure

```
dist/
└── linux/       # Linux desktop integration
    └── rustforged.desktop
```

## macOS

The macOS `.app`/`.dmg` metadata (bundle identifier, version, copyright, minimum
system version, `NSHighResolutionCapable`, etc.) is generated entirely by
cargo-packager from `[package.metadata.packager]` in the root `Cargo.toml`. There is
no hand-written `Info.plist` — cargo-packager builds a correct one from that config,
so the version can never drift out of sync with the crate version.

When code signing is set up, add the signing/notarization fields under
`[package.metadata.packager.macos]` in `Cargo.toml` (see `packaging/README.md`).

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
