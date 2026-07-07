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

# Load local signing/notarization config if present. .env is gitignored and holds
# APPLE_SIGNING_IDENTITY (+ optional APPLE_API_* notarization creds). Without it,
# cargo-packager produces an unsigned DMG — same as before this file existed.
if [[ -f "$PROJECT_ROOT/.env" ]]; then
    echo "Loading signing config from .env"
    set -a
    # shellcheck disable=SC1091
    source "$PROJECT_ROOT/.env"
    set +a

    # notarytool needs an absolute path to the .p8; resolve it relative to the repo root.
    if [[ -n "${APPLE_API_KEY_PATH:-}" && "${APPLE_API_KEY_PATH}" != /* ]]; then
        APPLE_API_KEY_PATH="$PROJECT_ROOT/$APPLE_API_KEY_PATH"
        export APPLE_API_KEY_PATH
    fi
fi

# Report what the build will do so there are no silent unsigned surprises.
# cargo-packager reads the signing identity from Cargo.toml, not the environment.
SIGNING_IDENTITY=$(grep -E '^[[:space:]]*signing-identity' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [[ -n "$SIGNING_IDENTITY" ]]; then
    echo "Signing as: $SIGNING_IDENTITY (from Cargo.toml)"
    if [[ -n "${APPLE_API_KEY_PATH:-}" ]]; then
        if [[ ! -f "$APPLE_API_KEY_PATH" ]]; then
            echo "Error: APPLE_API_KEY_PATH set but file not found: $APPLE_API_KEY_PATH"
            exit 1
        fi
        echo "Notarization: enabled (App Store Connect API key)"
    else
        echo "Notarization: disabled (signing only) — set APPLE_API_* in .env to notarize"
    fi
else
    echo "Signing: DISABLED (no signing-identity in Cargo.toml) — building an unsigned DMG"
fi

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

# Build a DMG for both Apple Silicon and Intel by default. Either architecture can be
# built from either kind of Mac (the Xcode toolchain and Rust both cross-compile).
# Pass one or more targets as args to override, e.g. for a quick native-only build:
#   ./scripts/build-macos.sh aarch64-apple-darwin
if [[ $# -gt 0 ]]; then
    TARGETS=("$@")
else
    TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
fi

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

# cargo-packager notarizes + staples the .app inside the DMG, but leaves the DMG
# itself un-notarized (it only signs it). A downloaded un-notarized DMG trips a
# Gatekeeper warning on open, so notarize + staple each DMG we just built. Requires
# the API key; skipped for signing-only or unsigned builds.
if [[ -n "$SIGNING_IDENTITY" && -n "${APPLE_API_KEY_PATH:-}" ]]; then
    for DMG in "target/release/packager/Rustforged_${VERSION}"_*.dmg; do
        [[ -f "$DMG" ]] || continue
        echo ""
        echo "Notarizing DMG: $(basename "$DMG")"
        xcrun notarytool submit "$DMG" \
            --key "$APPLE_API_KEY_PATH" \
            --key-id "$APPLE_API_KEY" \
            --issuer "$APPLE_API_ISSUER" \
            --wait
        echo "Stapling ticket to DMG..."
        xcrun stapler staple "$DMG"
    done
fi

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
