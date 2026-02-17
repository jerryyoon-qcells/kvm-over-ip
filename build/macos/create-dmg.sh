#!/usr/bin/env bash
# =============================================================================
# create-dmg.sh — Build a macOS .dmg Disk Image for KVM-Over-IP Client
# =============================================================================
#
# PURPOSE:
#   This script takes the kvm-client .app bundle produced by cargo-bundle and
#   wraps it in a .dmg disk image.  DMG is the standard distribution format
#   for macOS applications outside the App Store.  Users download the .dmg,
#   double-click it, drag the .app to /Applications, and the install is done.
#
# PREREQUISITES:
#   1. macOS (this script uses macOS-only tools: hdiutil, Finder)
#   2. Rust toolchain (cargo, cargo-bundle)
#      Install cargo-bundle: cargo install cargo-bundle
#   3. (Optional) create-dmg tool for a prettier DMG with background image
#      Install via Homebrew: brew install create-dmg
#      Without create-dmg, this script falls back to hdiutil (plain DMG).
#
# USAGE:
#   ./build/macos/create-dmg.sh [--version <version>] [--sign <identity>]
#
#   Options:
#     --version <version>  Override the version string (default: read from Cargo.toml)
#     --sign <identity>    Code sign with this Apple Developer identity
#                          Example: "Developer ID Application: Your Name (TEAMID)"
#     --help               Show this help message
#
# WHAT THIS SCRIPT DOES:
#   1. Builds the release binary with `cargo build --release`
#   2. Bundles the binary into a .app with `cargo bundle --release`
#   3. Copies Info.plist and entitlements from build/macos/ into the bundle
#   4. (Optional) Signs the .app bundle with codesign
#   5. Creates a .dmg disk image containing the .app
#
# OUTPUT:
#   dist/macos/kvm-over-ip-<version>-macos.dmg
#
# ERROR HANDLING:
#   The script uses `set -euo pipefail`:
#     -e  : Exit immediately if any command returns a non-zero exit code.
#     -u  : Treat unset variables as errors (prevents typos from silently
#           using empty strings).
#     -o pipefail: If any command in a pipeline fails, the whole pipeline fails.
#           Without this, `cmd1 | cmd2` would succeed even if cmd1 failed.
#
# CODE SIGNING GUIDANCE:
#   To distribute outside the App Store without Gatekeeper warnings:
#     1. Obtain a "Developer ID Application" certificate from Apple.
#     2. Pass --sign "Developer ID Application: Your Name (TEAMID)" to this script.
#     3. After building the DMG, notarise it with:
#          xcrun notarytool submit dist/macos/kvm-over-ip-*.dmg \
#                --apple-id your@email.com --team-id TEAMID --wait
#          xcrun stapler staple dist/macos/kvm-over-ip-*.dmg
#   Without signing, macOS Gatekeeper will block launch on first open.
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Colour output helpers (only when connected to a real terminal)
# ---------------------------------------------------------------------------
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'   # No Colour
else
    RED='' GREEN='' YELLOW='' BLUE='' NC=''
fi

# Print a status message.
info()    { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[OK]${NC}  $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }
# Print an error message and exit.
error()   { echo -e "${RED}[ERR]${NC}  $*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Default configuration
# ---------------------------------------------------------------------------

# Read version from workspace Cargo.toml (src/Cargo.toml).
# The pattern matches: version = "0.1.0" under [workspace.package]
DEFAULT_VERSION=$(grep -m1 '^version' src/Cargo.toml 2>/dev/null \
    | sed 's/version = "\(.*\)"/\1/' || echo "0.1.0")

APP_VERSION="${DEFAULT_VERSION}"
SIGN_IDENTITY=""     # Empty = do not sign
APP_NAME="KVM-Over-IP Client"
BUNDLE_ID="com.your-org.kvm-over-ip.client"
CRATE_NAME="kvm-client"

# Absolute paths (script can be run from any directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Workspace root is two levels above build/macos/
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DIST_DIR="${WORKSPACE_ROOT}/dist/macos"
APP_BUNDLE_NAME="${APP_NAME}.app"

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

usage() {
    echo "Usage: $0 [--version <version>] [--sign <identity>] [--help]"
    echo ""
    echo "Options:"
    echo "  --version <version>  Version string for the DMG filename (default: ${DEFAULT_VERSION})"
    echo "  --sign <identity>    Apple Developer ID identity for codesign"
    echo "  --help               Show this help message"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            APP_VERSION="$2"
            shift 2
            ;;
        --sign)
            SIGN_IDENTITY="$2"
            shift 2
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            error "Unknown argument: $1.  Run with --help for usage."
            ;;
    esac
done

DMG_NAME="kvm-over-ip-${APP_VERSION}-macos"
DMG_PATH="${DIST_DIR}/${DMG_NAME}.dmg"

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------

info "Starting KVM-Over-IP macOS packaging (version ${APP_VERSION})"

# Verify we are on macOS.
if [[ "$(uname -s)" != "Darwin" ]]; then
    error "This script must be run on macOS.  Current OS: $(uname -s)"
fi

# Verify cargo is available.
if ! command -v cargo &>/dev/null; then
    error "cargo not found.  Install Rust from https://rustup.rs/"
fi

# Verify cargo-bundle is installed.
if ! cargo bundle --version &>/dev/null 2>&1; then
    error "cargo-bundle not found.  Install with: cargo install cargo-bundle"
fi

# ---------------------------------------------------------------------------
# Step 1: Build the release binary
# ---------------------------------------------------------------------------

info "Building kvm-client in release mode..."
cargo build \
    --manifest-path "${WORKSPACE_ROOT}/src/Cargo.toml" \
    --release \
    --package kvm-client
success "Binary built: src/target/release/kvm-client"

# ---------------------------------------------------------------------------
# Step 2: Create the .app bundle with cargo-bundle
# ---------------------------------------------------------------------------

info "Creating .app bundle with cargo-bundle..."
# cargo-bundle must be run from the workspace root so it can find Cargo.toml.
# It reads [package.metadata.bundle] from src/crates/kvm-client/Cargo.toml.
(
    cd "${WORKSPACE_ROOT}/src"
    cargo bundle --release --package kvm-client
)

# cargo-bundle places the bundle at: src/target/release/bundle/osx/<AppName>.app
BUNDLE_SRC="${WORKSPACE_ROOT}/src/target/release/bundle/osx/${APP_BUNDLE_NAME}"

if [[ ! -d "${BUNDLE_SRC}" ]]; then
    error "Bundle not found at expected path: ${BUNDLE_SRC}"
fi
success ".app bundle created at: ${BUNDLE_SRC}"

# ---------------------------------------------------------------------------
# Step 3: Inject our custom Info.plist (with template variable substitution)
# ---------------------------------------------------------------------------

info "Injecting custom Info.plist..."

# Determine the SHORT_VERSION (major.minor from the full version string).
# Example: "0.1.0" → "0.1"
SHORT_VERSION=$(echo "${APP_VERSION}" | cut -d. -f1-2)

# Substitute template variables in Info.plist and write into the bundle.
# The sed -e flags apply multiple substitutions.
sed \
    -e "s/{{BUNDLE_VERSION}}/${APP_VERSION}/g" \
    -e "s/{{SHORT_VERSION}}/${SHORT_VERSION}/g" \
    "${SCRIPT_DIR}/Info.plist" \
    > "${BUNDLE_SRC}/Contents/Info.plist"

success "Info.plist written to bundle."

# ---------------------------------------------------------------------------
# Step 4: Copy entitlements into the bundle (for reference during signing)
# ---------------------------------------------------------------------------

# Note: entitlements are NOT placed inside the bundle — they are passed to
# codesign as an external file.  We copy it next to the bundle for convenience.
cp "${SCRIPT_DIR}/entitlements.plist" "${WORKSPACE_ROOT}/src/target/release/bundle/osx/"

# ---------------------------------------------------------------------------
# Step 5: (Optional) Code sign the .app bundle
# ---------------------------------------------------------------------------

if [[ -n "${SIGN_IDENTITY}" ]]; then
    info "Signing .app bundle with identity: ${SIGN_IDENTITY}"
    codesign \
        --deep \
        --force \
        --sign "${SIGN_IDENTITY}" \
        --entitlements "${SCRIPT_DIR}/entitlements.plist" \
        --options runtime \
        "${BUNDLE_SRC}"
    success "Code signing complete."
else
    warn "Skipping code signing (no --sign identity provided)."
    warn "Unsigned apps will be blocked by Gatekeeper on first launch."
    warn "Users can override this with: xattr -dr com.apple.quarantine '${APP_BUNDLE_NAME}'"
fi

# ---------------------------------------------------------------------------
# Step 6: Create the DMG disk image
# ---------------------------------------------------------------------------

# Create the output directory.
mkdir -p "${DIST_DIR}"

info "Creating DMG disk image..."

# Prefer the 'create-dmg' tool if available — it produces a polished DMG
# with a background image and an Applications folder shortcut.
if command -v create-dmg &>/dev/null; then
    info "Using 'create-dmg' for a polished DMG..."
    create-dmg \
        --volname "KVM-Over-IP ${APP_VERSION}" \
        --volicon "${SCRIPT_DIR}/AppIcon.icns" 2>/dev/null \
        --window-pos 200 120 \
        --window-size 800 450 \
        --icon-size 128 \
        --icon "${APP_BUNDLE_NAME}" 200 190 \
        --hide-extension "${APP_BUNDLE_NAME}" \
        --app-drop-link 600 185 \
        "${DMG_PATH}" \
        "${BUNDLE_SRC}" || {
            warn "create-dmg failed, falling back to hdiutil..."
            _create_hdiutil_dmg
        }
else
    _create_hdiutil_dmg
fi

success "DMG created: ${DMG_PATH}"

# ---------------------------------------------------------------------------
# Step 7: Print next steps
# ---------------------------------------------------------------------------

info "Done!  Packaging complete."
echo ""
echo "Output: ${DMG_PATH}"
echo ""

if [[ -n "${SIGN_IDENTITY}" ]]; then
    echo "Next steps for notarisation:"
    echo "  xcrun notarytool submit '${DMG_PATH}' \\"
    echo "        --apple-id your@email.com --team-id YOUR_TEAM_ID --wait"
    echo "  xcrun stapler staple '${DMG_PATH}'"
else
    echo "To distribute without Gatekeeper warnings, you must:"
    echo "  1. Sign with an Apple Developer ID certificate"
    echo "  2. Notarise with Apple's notarisation service"
    echo "  See docs/PACKAGING.md for detailed instructions."
fi

# ---------------------------------------------------------------------------
# Helper: create a plain DMG using hdiutil (fallback when create-dmg absent)
# ---------------------------------------------------------------------------
_create_hdiutil_dmg() {
    # Create a read-write temporary DMG first, then convert it to
    # a compressed read-only DMG (UDZO = zlib-compressed UDIF).

    local TMP_DMG="${DIST_DIR}/${DMG_NAME}-rw.dmg"
    local MOUNT_POINT="/Volumes/KVM-Over-IP_tmp"

    info "Creating temporary read-write DMG with hdiutil..."

    # Calculate the size of the .app bundle in MB and add 20% headroom.
    local SIZE_MB
    SIZE_MB=$(du -sm "${BUNDLE_SRC}" | cut -f1)
    SIZE_MB=$(( SIZE_MB * 6 / 5 + 10 ))

    hdiutil create \
        -size "${SIZE_MB}m" \
        -fs "HFS+" \
        -volname "KVM-Over-IP ${APP_VERSION}" \
        -format UDRW \
        "${TMP_DMG}"

    # Mount the temporary DMG.
    hdiutil attach "${TMP_DMG}" -mountpoint "${MOUNT_POINT}"

    # Copy the .app bundle into the mounted volume.
    cp -R "${BUNDLE_SRC}" "${MOUNT_POINT}/"

    # Create an Applications symlink so users can drag-and-drop install.
    ln -s /Applications "${MOUNT_POINT}/Applications"

    # Unmount.
    hdiutil detach "${MOUNT_POINT}"

    # Convert to compressed read-only DMG.
    hdiutil convert "${TMP_DMG}" -format UDZO -o "${DMG_PATH}"
    rm -f "${TMP_DMG}"
}
