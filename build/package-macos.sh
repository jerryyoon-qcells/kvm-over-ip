#!/usr/bin/env bash
# =============================================================================
# package-macos.sh — macOS Local Packaging Script for KVM-Over-IP
# =============================================================================
#
# PURPOSE:
#   This script builds the macOS .app bundle and .dmg disk image for
#   kvm-client on a local Mac.  Use it to test macOS packaging before
#   pushing a release tag to GitHub.
#
# PREREQUISITES:
#   1. macOS (Monterey 12.0 or later recommended)
#   2. Xcode Command Line Tools
#      Install with: xcode-select --install
#   3. Rust stable toolchain
#      Install from: https://rustup.rs/
#   4. cargo-bundle
#      Install with: cargo install cargo-bundle
#   5. (Optional but recommended) create-dmg for a polished DMG
#      Install with: brew install create-dmg
#   6. (Optional) Inkscape or ImageMagick for converting AppIcon.svg → .icns
#      Install Inkscape: brew install inkscape
#
# USAGE:
#   ./build/package-macos.sh
#   ./build/package-macos.sh --skip-build             # Use existing binaries
#   ./build/package-macos.sh --sign "Developer ID Application: Name (TEAMID)"
#   ./build/package-macos.sh --version 1.2.3          # Override version
#   ./build/package-macos.sh --help
#
# WHAT THIS SCRIPT DOES:
#   1. Verifies all prerequisites are installed.
#   2. (Optional) Converts AppIcon.svg to AppIcon.icns.
#   3. Builds the release binary with cargo build --release.
#   4. Creates a .app bundle with cargo-bundle.
#   5. Injects the custom Info.plist (with version substitution).
#   6. (Optional) Signs the .app with codesign.
#   7. Creates a .dmg disk image.
#   8. Copies the .dmg to dist/macos/.
#
# OUTPUT:
#   dist/macos/kvm-over-ip-<version>-macos.dmg
#
# ERROR HANDLING:
#   set -euo pipefail exits immediately on any error.
#   All failures are reported with a [ERR] prefix and a meaningful message.
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Colour output helpers
# ---------------------------------------------------------------------------
if [ -t 1 ]; then
    RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
    BLUE='\033[0;34m'; NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' NC=''
fi

info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()      { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERR]${NC}   $*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------

SKIP_BUILD=false
SIGN_IDENTITY=""
VERSION_OVERRIDE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-build)  SKIP_BUILD=true; shift ;;
        --sign)        SIGN_IDENTITY="$2"; shift 2 ;;
        --version)     VERSION_OVERRIDE="$2"; shift 2 ;;
        --help)
            echo "Usage: $0 [--skip-build] [--sign <identity>] [--version <ver>] [--help]"
            echo ""
            echo "Options:"
            echo "  --skip-build          Skip cargo build; use existing target/release/kvm-client"
            echo "  --sign <identity>     Code sign with this Apple Developer ID identity"
            echo "  --version <version>   Override the version string (default: from Cargo.toml)"
            exit 0
            ;;
        *)
            error "Unknown argument: $1.  Run with --help for usage."
            ;;
    esac
done

# ---------------------------------------------------------------------------
# Locate directories
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SRC_DIR="${WORKSPACE_ROOT}/src"
MACOS_BUILD_DIR="${SCRIPT_DIR}/macos"
DIST_DIR="${WORKSPACE_ROOT}/dist/macos"

info "Workspace root:  ${WORKSPACE_ROOT}"
info "Rust workspace:  ${SRC_DIR}"
info "macOS build dir: ${MACOS_BUILD_DIR}"
info "Output dir:      ${DIST_DIR}"
echo ""

# ---------------------------------------------------------------------------
# Determine version
# ---------------------------------------------------------------------------

if [[ -n "${VERSION_OVERRIDE}" ]]; then
    APP_VERSION="${VERSION_OVERRIDE}"
else
    # Extract version from workspace Cargo.toml.
    # This reads the first `version = "..."` line under [workspace.package].
    APP_VERSION=$(grep -m1 '^version' "${SRC_DIR}/Cargo.toml" \
        | sed 's/version *= *"\(.*\)"/\1/' || echo "0.1.0")
fi

SHORT_VERSION=$(echo "${APP_VERSION}" | cut -d. -f1-2)
DMG_NAME="kvm-over-ip-${APP_VERSION}-macos"
APP_BUNDLE_NAME="KVM-Over-IP Client.app"

info "Building version: ${APP_VERSION}"
echo ""

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------

info "Checking prerequisites..."

# Verify we are on macOS.
if [[ "$(uname -s)" != "Darwin" ]]; then
    error "This script must be run on macOS.  Current OS: $(uname -s)"
fi

# Verify Xcode Command Line Tools are installed.
if ! xcode-select -p &>/dev/null; then
    error "Xcode Command Line Tools not found.  Install with: xcode-select --install"
fi
ok "Xcode CLT found: $(xcode-select -p)"

# Verify cargo is available.
if ! command -v cargo &>/dev/null; then
    error "cargo not found.  Install Rust from https://rustup.rs/"
fi
ok "cargo found: $(cargo --version)"

# Verify cargo-bundle is installed.
if ! cargo bundle --version &>/dev/null 2>&1; then
    error "cargo-bundle not found.  Install with: cargo install cargo-bundle"
fi
ok "cargo-bundle found."

# Warn if create-dmg is not available (we fall back to hdiutil).
if ! command -v create-dmg &>/dev/null; then
    warn "create-dmg not found.  Using hdiutil for a plain DMG."
    warn "For a polished DMG with background art, install: brew install create-dmg"
fi

# Verify the workspace Cargo.toml.
if ! [ -f "${SRC_DIR}/Cargo.toml" ]; then
    error "Cargo.toml not found at: ${SRC_DIR}/Cargo.toml"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 1: (Optional) Convert SVG icon to .icns
# ---------------------------------------------------------------------------

ICNS_FILE="${MACOS_BUILD_DIR}/AppIcon.icns"
SVG_FILE="${MACOS_BUILD_DIR}/AppIcon.svg"

if [ -f "${ICNS_FILE}" ]; then
    ok "AppIcon.icns already exists — skipping icon conversion."
elif [ -f "${SVG_FILE}" ] && command -v inkscape &>/dev/null; then
    info "Converting AppIcon.svg → AppIcon.icns using Inkscape..."

    ICONSET_DIR="${MACOS_BUILD_DIR}/AppIcon.iconset"
    mkdir -p "${ICONSET_DIR}"

    # Generate PNG files at all required sizes.
    # macOS icon sizes: 16, 32, 64, 128, 256, 512, 1024 pixels.
    for size in 16 32 64 128 256 512 1024; do
        inkscape \
            --export-filename="${ICONSET_DIR}/icon_${size}.png" \
            -w "${size}" -h "${size}" \
            "${SVG_FILE}" 2>/dev/null
    done

    # Arrange into the iconset with the names macOS expects.
    cp "${ICONSET_DIR}/icon_16.png"   "${ICONSET_DIR}/icon_16x16.png"
    cp "${ICONSET_DIR}/icon_32.png"   "${ICONSET_DIR}/icon_16x16@2x.png"
    cp "${ICONSET_DIR}/icon_32.png"   "${ICONSET_DIR}/icon_32x32.png"
    cp "${ICONSET_DIR}/icon_64.png"   "${ICONSET_DIR}/icon_32x32@2x.png"
    cp "${ICONSET_DIR}/icon_128.png"  "${ICONSET_DIR}/icon_128x128.png"
    cp "${ICONSET_DIR}/icon_256.png"  "${ICONSET_DIR}/icon_128x128@2x.png"
    cp "${ICONSET_DIR}/icon_256.png"  "${ICONSET_DIR}/icon_256x256.png"
    cp "${ICONSET_DIR}/icon_512.png"  "${ICONSET_DIR}/icon_256x256@2x.png"
    cp "${ICONSET_DIR}/icon_512.png"  "${ICONSET_DIR}/icon_512x512.png"
    cp "${ICONSET_DIR}/icon_1024.png" "${ICONSET_DIR}/icon_512x512@2x.png"

    # Convert to .icns using Apple's iconutil.
    iconutil -c icns "${ICONSET_DIR}" -o "${ICNS_FILE}"
    rm -rf "${ICONSET_DIR}"

    ok "AppIcon.icns created."
else
    warn "AppIcon.icns not found and icon conversion skipped."
    warn "The .app bundle will use the default system icon."
    warn "To generate an icon, install Inkscape (brew install inkscape)"
    warn "and place a source SVG at: ${SVG_FILE}"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 2: Build release binary
# ---------------------------------------------------------------------------

if $SKIP_BUILD; then
    warn "Skipping cargo build (--skip-build specified)."
else
    info "Building kvm-client in release mode..."
    cargo build \
        --manifest-path "${SRC_DIR}/Cargo.toml" \
        --release \
        --package kvm-client \
        || error "cargo build failed with exit code $?"
    ok "Binary built: src/target/release/kvm-client"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 3: Create the .app bundle with cargo-bundle
# ---------------------------------------------------------------------------

info "Creating .app bundle with cargo-bundle..."

(
    cd "${SRC_DIR}"
    cargo bundle --release --package kvm-client \
        || error "cargo-bundle failed with exit code $?"
)

BUNDLE_PATH="${SRC_DIR}/target/release/bundle/osx/${APP_BUNDLE_NAME}"

if ! [ -d "${BUNDLE_PATH}" ]; then
    error "Bundle not found at expected path: ${BUNDLE_PATH}"
fi
ok ".app bundle created: ${BUNDLE_PATH}"
echo ""

# ---------------------------------------------------------------------------
# Step 4: Inject custom Info.plist
# ---------------------------------------------------------------------------

info "Injecting custom Info.plist (version ${APP_VERSION})..."

sed \
    -e "s/{{BUNDLE_VERSION}}/${APP_VERSION}/g" \
    -e "s/{{SHORT_VERSION}}/${SHORT_VERSION}/g" \
    "${MACOS_BUILD_DIR}/Info.plist" \
    > "${BUNDLE_PATH}/Contents/Info.plist"

ok "Info.plist injected."
echo ""

# ---------------------------------------------------------------------------
# Step 5: (Optional) Copy .icns icon into the bundle
# ---------------------------------------------------------------------------

if [ -f "${ICNS_FILE}" ]; then
    RESOURCES_DIR="${BUNDLE_PATH}/Contents/Resources"
    mkdir -p "${RESOURCES_DIR}"
    cp "${ICNS_FILE}" "${RESOURCES_DIR}/AppIcon.icns"
    ok "AppIcon.icns copied into bundle."
fi

# ---------------------------------------------------------------------------
# Step 6: (Optional) Code sign the .app bundle
# ---------------------------------------------------------------------------

if [[ -n "${SIGN_IDENTITY}" ]]; then
    info "Signing .app bundle with identity: ${SIGN_IDENTITY}"

    codesign \
        --deep \
        --force \
        --sign "${SIGN_IDENTITY}" \
        --entitlements "${MACOS_BUILD_DIR}/entitlements.plist" \
        --options runtime \
        "${BUNDLE_PATH}" \
        || error "codesign failed with exit code $?"

    ok "Code signing complete."
    echo ""
else
    warn "Skipping code signing (no --sign identity provided)."
    warn "Unsigned apps will trigger Gatekeeper warnings on first launch."
    echo ""
fi

# ---------------------------------------------------------------------------
# Step 7: Create the .dmg disk image
# ---------------------------------------------------------------------------

mkdir -p "${DIST_DIR}"
DMG_PATH="${DIST_DIR}/${DMG_NAME}.dmg"

info "Creating .dmg disk image..."

if command -v create-dmg &>/dev/null; then
    info "Using create-dmg for a polished DMG..."
    # create-dmg produces a DMG with a background image and drag-to-Applications UI.
    create-dmg \
        --volname "KVM-Over-IP ${APP_VERSION}" \
        --window-pos 200 120 \
        --window-size 800 450 \
        --icon-size 128 \
        --icon "${APP_BUNDLE_NAME}" 200 190 \
        --hide-extension "${APP_BUNDLE_NAME}" \
        --app-drop-link 600 185 \
        "${DMG_PATH}" \
        "${BUNDLE_PATH}" \
        || { warn "create-dmg failed, falling back to hdiutil..."; _hdiutil_dmg; }
else
    _hdiutil_dmg
fi

ok "DMG created: ${DMG_PATH}"
echo ""

# ---------------------------------------------------------------------------
# Step 8: Summary
# ---------------------------------------------------------------------------

ok "Packaging complete!"
echo ""
info "Output: ${DMG_PATH}"
SIZE=$(du -h "${DMG_PATH}" | cut -f1)
info "Size:   ${SIZE}"
echo ""

if [[ -n "${SIGN_IDENTITY}" ]]; then
    info "Next steps — notarisation:"
    info "  xcrun notarytool submit '${DMG_PATH}' \\"
    info "        --apple-id your@email.com --team-id YOUR_TEAM_ID --wait"
    info "  xcrun stapler staple '${DMG_PATH}'"
else
    warn "To distribute without Gatekeeper warnings:"
    warn "  1. Obtain a Developer ID Application certificate from Apple"
    warn "  2. Re-run with: --sign 'Developer ID Application: Your Name (TEAMID)'"
    warn "  3. Notarise the signed DMG (see docs/PACKAGING.md)"
fi

# ---------------------------------------------------------------------------
# Helper: create a plain DMG with hdiutil (fallback)
# ---------------------------------------------------------------------------
_hdiutil_dmg() {
    info "Creating plain DMG with hdiutil..."

    local TMP_DMG="${DIST_DIR}/${DMG_NAME}-rw.dmg"
    local MOUNT_POINT="/Volumes/KVMOverIP_pkg"

    # Calculate the bundle size with 20% headroom.
    local SIZE_MB
    SIZE_MB=$(du -sm "${BUNDLE_PATH}" | cut -f1)
    SIZE_MB=$(( SIZE_MB * 6 / 5 + 10 ))

    # Create a writable disk image.
    hdiutil create \
        -size "${SIZE_MB}m" \
        -fs "HFS+" \
        -volname "KVM-Over-IP ${APP_VERSION}" \
        -format UDRW \
        "${TMP_DMG}"

    # Mount, populate, and unmount.
    hdiutil attach "${TMP_DMG}" -mountpoint "${MOUNT_POINT}"
    cp -R "${BUNDLE_PATH}" "${MOUNT_POINT}/"
    ln -s /Applications "${MOUNT_POINT}/Applications"
    hdiutil detach "${MOUNT_POINT}"

    # Convert to compressed read-only UDZO (zlib-compressed UDIF).
    hdiutil convert "${TMP_DMG}" -format UDZO -o "${DMG_PATH}"
    rm -f "${TMP_DMG}"
}
