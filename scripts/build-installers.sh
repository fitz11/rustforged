#!/bin/bash
# Unified build script for Rustforged installers
# Detects the current platform and builds the appropriate installer

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VERSION=$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')

echo "Rustforged v$VERSION Installer Builder"
echo "======================================"
echo ""

# Detect OS
case "$(uname -s)" in
    Darwin)
        echo "Detected: macOS"
        echo "Building DMG installer..."
        echo ""
        exec "$SCRIPT_DIR/build-macos.sh"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "Detected: Windows (Git Bash/MSYS)"
        echo ""
        echo "For Windows builds, use PowerShell:"
        echo "  .\\scripts\\build-windows.ps1"
        echo ""
        echo "Options:"
        echo "  .\\scripts\\build-windows.ps1 -Architecture x64"
        echo "  .\\scripts\\build-windows.ps1 -Architecture arm64"
        echo "  .\\scripts\\build-windows.ps1 -Architecture both"
        exit 0
        ;;
    Linux)
        echo "Detected: Linux"
        echo ""
        echo "Linux users build from source. Available targets:"
        echo ""
        echo "  macOS (run on Mac):"
        echo "    ./scripts/build-macos.sh"
        echo ""
        echo "  Windows (run on Windows):"
        echo "    .\\scripts\\build-windows.ps1"
        echo ""
        echo "To build a Linux release tarball:"
        echo "  cargo build --release"
        echo "  mkdir -p dist/rustforged"
        echo "  cp target/release/rustforged dist/rustforged/"
        echo "  cp -r assets dist/rustforged/"
        echo "  tar -czvf rustforged-linux-x86_64.tar.gz -C dist rustforged"
        exit 0
        ;;
    *)
        echo "Unknown platform: $(uname -s)"
        echo "Supported platforms: macOS, Windows"
        exit 1
        ;;
esac
