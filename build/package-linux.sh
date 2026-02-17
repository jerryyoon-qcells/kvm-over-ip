#!/usr/bin/env bash
# =============================================================================
# package-linux.sh — Linux Local Packaging Script for KVM-Over-IP
# =============================================================================
#
# PURPOSE:
#   This script builds Debian .deb packages for kvm-client and kvm-web-bridge
#   on a local Linux machine.  Use it to test .deb packaging before pushing a
#   release tag to GitHub.
#
# PREREQUISITES:
#   1. A Debian-based Linux distribution (Ubuntu, Debian, Linux Mint, etc.)
#      On non-Debian distros, cargo-deb still works but `apt install` will not.
#   2. Rust stable toolchain
#      Install from: https://rustup.rs/
#   3. cargo-deb
#      Install with: cargo install cargo-deb
#   4. X11 development libraries (required to compile kvm-client)
#      Install with: sudo apt install libx11-dev libxtst-dev
#
# USAGE:
#   ./build/package-linux.sh
#   ./build/package-linux.sh --skip-build     # Use existing compiled binaries
#   ./build/package-linux.sh --only-client    # Build only kvm-client .deb
#   ./build/package-linux.sh --only-bridge    # Build only kvm-web-bridge .deb
#   ./build/package-linux.sh --help
#
# OUTPUT:
#   .deb packages are placed in: src/target/debian/
#   A dist/linux/ directory is also created containing copies of the .deb files.
#
# ERROR HANDLING:
#   set -euo pipefail causes the script to exit immediately on any error.
#   All errors are reported with a clear [ERR] prefix and the exit code.
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Colour output helpers (only when writing to a real terminal)
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
# Print error and exit with non-zero code.
error()   { echo -e "${RED}[ERR]${NC}   $*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------

SKIP_BUILD=false
ONLY_CLIENT=false
ONLY_BRIDGE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-build)   SKIP_BUILD=true;  shift ;;
        --only-client)  ONLY_CLIENT=true; shift ;;
        --only-bridge)  ONLY_BRIDGE=true; shift ;;
        --help)
            echo "Usage: $0 [--skip-build] [--only-client] [--only-bridge] [--help]"
            exit 0
            ;;
        *)
            error "Unknown argument: $1.  Run with --help for usage."
            ;;
    esac
done

if $ONLY_CLIENT && $ONLY_BRIDGE; then
    error "--only-client and --only-bridge cannot be used together."
fi

# ---------------------------------------------------------------------------
# Locate directories
# ---------------------------------------------------------------------------

# SCRIPT_DIR is the absolute path to the build/ directory.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Workspace root is the parent of build/.
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SRC_DIR="${WORKSPACE_ROOT}/src"
DIST_DIR="${WORKSPACE_ROOT}/dist/linux"

info "Workspace root: ${WORKSPACE_ROOT}"
info "Rust workspace: ${SRC_DIR}"
info "Output dir:     ${DIST_DIR}"
echo ""

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------

info "Checking prerequisites..."

# Verify we are on Linux.
if [[ "$(uname -s)" != "Linux" ]]; then
    error "This script must be run on Linux.  Current OS: $(uname -s)"
fi

# Verify we are on a Debian-based distribution (for apt/dpkg availability).
if ! command -v dpkg &>/dev/null; then
    warn "dpkg not found.  This system may not be Debian-based."
    warn "The .deb packages can still be built but cannot be installed with apt."
fi

# Verify cargo is available.
if ! command -v cargo &>/dev/null; then
    error "cargo not found.  Install Rust from https://rustup.rs/"
fi
ok "cargo found: $(cargo --version)"

# Verify cargo-deb is installed.
if ! cargo deb --version &>/dev/null 2>&1; then
    error "cargo-deb not found.  Install with: cargo install cargo-deb"
fi
ok "cargo-deb found: $(cargo deb --version)"

# Verify X11 development libraries are present (needed to compile kvm-client).
# We check for the presence of X11/Xlib.h which is provided by libx11-dev.
if ! [ -f /usr/include/X11/Xlib.h ] && ! pkg-config --exists x11 2>/dev/null; then
    warn "X11 development headers not found."
    warn "Install with: sudo apt install libx11-dev libxtst-dev"
    warn "Attempting to build anyway — compilation will fail if headers are missing."
fi

# Verify the workspace Cargo.toml exists.
if ! [ -f "${SRC_DIR}/Cargo.toml" ]; then
    error "Cargo.toml not found at: ${SRC_DIR}/Cargo.toml"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 1: Build release binaries
# ---------------------------------------------------------------------------

if $SKIP_BUILD; then
    warn "Skipping cargo build (--skip-build specified)."
    warn "Using existing binaries in src/target/release/"
else
    info "Building release binaries (cargo build --release --workspace)..."
    cargo build \
        --manifest-path "${SRC_DIR}/Cargo.toml" \
        --release \
        --workspace \
        || error "cargo build failed with exit code $?"
    ok "Release binaries built."
fi

echo ""

# ---------------------------------------------------------------------------
# Step 2: Build .deb packages
# ---------------------------------------------------------------------------

# Helper function to build one .deb package.
# Arguments: $1 = package name
build_deb() {
    local package_name="$1"
    info "Building ${package_name} .deb package..."

    # Verify the crate directory exists.
    local crate_dir="${SRC_DIR}/crates/${package_name}"
    if ! [ -d "${crate_dir}" ]; then
        error "Crate directory not found: ${crate_dir}"
    fi

    # Run cargo-deb.
    # --manifest-path: path to workspace Cargo.toml (cargo-deb finds the
    #                  package within the workspace using --package).
    # --package:       which workspace member to package.
    # --no-build:      use existing compiled binary, skip recompilation.
    cargo deb \
        --manifest-path "${SRC_DIR}/Cargo.toml" \
        --package "${package_name}" \
        --no-build \
        || error "cargo-deb for ${package_name} failed with exit code $?"

    ok "${package_name} .deb built."
}

if ! $ONLY_BRIDGE; then
    build_deb "kvm-client"
fi

if ! $ONLY_CLIENT; then
    build_deb "kvm-web-bridge"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 3: Collect artefacts into dist/linux/
# ---------------------------------------------------------------------------

info "Collecting .deb files into ${DIST_DIR} ..."
mkdir -p "${DIST_DIR}"

# cargo-deb places .deb files in src/target/debian/
DEB_OUTPUT_DIR="${SRC_DIR}/target/debian"

if [ -d "${DEB_OUTPUT_DIR}" ]; then
    # Copy each .deb file to dist/linux/ and print its size.
    find "${DEB_OUTPUT_DIR}" -name "*.deb" | while read -r deb_file; do
        cp "${deb_file}" "${DIST_DIR}/"
        ok "Copied: $(basename "${deb_file}") → dist/linux/"
    done
else
    warn "cargo-deb output directory not found: ${DEB_OUTPUT_DIR}"
fi

echo ""

# ---------------------------------------------------------------------------
# Step 4: Summary
# ---------------------------------------------------------------------------

ok "Packaging complete!"
echo ""
info "Output directory: ${DIST_DIR}"

if ls "${DIST_DIR}"/*.deb &>/dev/null; then
    ls -lh "${DIST_DIR}"/*.deb | awk '{print "  " $5 "\t" $9}'
    echo ""
    info "To install on this machine:"
    info "  sudo apt install ./dist/linux/kvm-client_*.deb"
    info "  sudo apt install ./dist/linux/kvm-web-bridge_*.deb"
    echo ""
    info "To test the package without installing:"
    info "  dpkg --info dist/linux/kvm-client_*.deb"
    info "  dpkg --contents dist/linux/kvm-client_*.deb"
else
    warn "No .deb files found in ${DIST_DIR}"
fi

echo ""
warn "NOTE: The .deb packages are unsigned."
warn "For distribution via apt repositories, sign with GPG:"
warn "  dpkg-sig --sign builder dist/linux/kvm-client_*.deb"
warn "See docs/PACKAGING.md for code signing instructions."
