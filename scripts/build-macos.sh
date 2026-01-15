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

# Determine architecture
ARCH=$(uname -m)
if [[ "$ARCH" == "arm64" ]]; then
    TARGET="aarch64-apple-darwin"
    ARCH_NAME="arm64"
else
    TARGET="x86_64-apple-darwin"
    ARCH_NAME="x64"
fi

echo "Target: $TARGET"

# Ensure target is installed
if ! rustup target list --installed | grep -q "$TARGET"; then
    echo "Installing Rust target: $TARGET"
    rustup target add "$TARGET"
fi

# Build release binary
echo ""
echo "Building release binary..."
cargo build --release --target "$TARGET"

# Create DMG installer
echo ""
echo "Creating DMG installer..."
cargo packager --release --target "$TARGET" --formats dmg

# Find and report output
DMG_DIR="target/$TARGET/release/dmg"
if [[ -d "$DMG_DIR" ]]; then
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
    echo "Error: DMG output directory not found"
    exit 1
fi
