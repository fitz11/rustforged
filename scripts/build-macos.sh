#!/bin/bash
# Build macOS DMG installer for Rustforged
# Run this script on a Mac

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VERSION=$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')

cd "$PROJECT_ROOT"

echo "Building Rustforged v$VERSION for macOS"
echo "========================================"

# Check prerequisites
echo "Checking prerequisites..."

if ! command -v cargo-packager &> /dev/null; then
    echo "Error: cargo-packager not found"
    echo "Install with: cargo install cargo-packager --locked"
    exit 1
fi

if [[ ! -f "packaging/icons/icon.icns" ]]; then
    echo "Error: macOS icon not found at packaging/icons/icon.icns"
    echo "See packaging/icons/README.md for icon generation instructions"
    exit 1
fi

# Build a DMG for both Apple Silicon and Intel. Either architecture can be built from
# either kind of Mac (the Xcode toolchain and Rust both cross-compile between them).
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)

for TARGET in "${TARGETS[@]}"; do
    echo ""
    echo "=== $TARGET ==="

    # Ensure target is installed
    if ! rustup target list --installed | grep -q "$TARGET"; then
        echo "Installing Rust target: $TARGET"
        rustup target add "$TARGET"
    fi

    echo "Building release binary..."
    cargo build --release --target "$TARGET"

    echo "Creating DMG installer..."
    cargo packager --release --target "$TARGET" --binaries-dir "target/$TARGET/release" --formats dmg
done

# Find and report output (aarch64 -> *_aarch64.dmg, x86_64 -> *_x64.dmg)
DMG_DIR="target/release/packager"
if [[ -d "$DMG_DIR" ]] && ls "$DMG_DIR"/*.dmg 1> /dev/null 2>&1; then
    echo ""
    echo "Build complete!"
    echo "Output:"
    ls -lh "$DMG_DIR"/*.dmg

    # Copy to releases directory
    mkdir -p "$PROJECT_ROOT/releases"
    cp "$DMG_DIR"/*.dmg "$PROJECT_ROOT/releases/"
    echo ""
    echo "Copied to releases/"
else
    echo "Error: DMG output not found in $DMG_DIR"
    exit 1
fi
