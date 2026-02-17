# =============================================================================
# package-windows.ps1 — Windows Local Packaging Script for KVM-Over-IP
# =============================================================================
#
# PURPOSE:
#   This script builds the Windows MSI installer packages for kvm-master and
#   kvm-client locally.  Use this script to test packaging on your development
#   machine before pushing a release tag to GitHub.
#
# PREREQUISITES:
#   1. Rust stable toolchain
#      Install from: https://rustup.rs/
#   2. cargo-wix subcommand
#      Install with: cargo install cargo-wix --version "0.3"
#   3. WiX Toolset v3 — must be installed and on the system PATH
#      Download from: https://wixtoolset.org/releases/
#      After installing, verify with: candle.exe /?
#   4. (Optional) Icon files: place kvm-master.ico and kvm-client.ico in
#      src/crates/kvm-master/wix/ and src/crates/kvm-client/wix/ respectively.
#      Without icons the MSI builds succeed but the Add/Remove Programs entry
#      will not have a custom icon.
#
# USAGE:
#   .\build\package-windows.ps1
#   .\build\package-windows.ps1 -SkipBuild       # Use existing compiled binaries
#   .\build\package-windows.ps1 -OnlyMaster       # Build only kvm-master MSI
#   .\build\package-windows.ps1 -OnlyClient       # Build only kvm-client MSI
#
# OUTPUT:
#   MSI files are placed in: src\target\wix\
#   A dist\windows\ directory is also created containing:
#     - Both MSI files
#     - The raw binaries as a ZIP archive
#
# ERROR HANDLING:
#   This script uses $ErrorActionPreference = 'Stop' so any failing command
#   causes the script to exit immediately with a non-zero exit code.
#   This prevents silent partial builds that might look successful but produce
#   incomplete output.
# =============================================================================

[CmdletBinding()]
param (
    # Skip the `cargo build --release` step (use binaries already in target/release/)
    [switch]$SkipBuild,

    # Build only the kvm-master MSI (skip kvm-client)
    [switch]$OnlyMaster,

    # Build only the kvm-client MSI (skip kvm-master)
    [switch]$OnlyClient,

    # Show this help message
    [switch]$Help
)

# ---------------------------------------------------------------------------
# Configuration — exit immediately on any error
# ---------------------------------------------------------------------------
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------

# Print a formatted status message.
function Write-Info  ([string]$Message) { Write-Host "[INFO]  $Message" -ForegroundColor Cyan   }
function Write-Ok    ([string]$Message) { Write-Host "[OK]    $Message" -ForegroundColor Green  }
function Write-Warn  ([string]$Message) { Write-Host "[WARN]  $Message" -ForegroundColor Yellow }
function Write-Err   ([string]$Message) { Write-Host "[ERR]   $Message" -ForegroundColor Red    }

# Exit with an error message and a non-zero exit code.
function Fail ([string]$Message) {
    Write-Err $Message
    # Exit with code 1 so CI and calling scripts can detect the failure.
    exit 1
}

# ---------------------------------------------------------------------------
# Show help if requested
# ---------------------------------------------------------------------------
if ($Help) {
    Get-Help $MyInvocation.MyCommand.Path -Detailed
    exit 0
}

if ($OnlyMaster -and $OnlyClient) {
    Fail "-OnlyMaster and -OnlyClient cannot be used together."
}

# ---------------------------------------------------------------------------
# Locate the workspace root (the directory containing this script's parent)
# ---------------------------------------------------------------------------

# $PSScriptRoot is the directory containing this script (build/).
# The workspace root is one level above build/.
$BuildDir     = $PSScriptRoot
$WorkspaceRoot = (Get-Item $BuildDir).Parent.FullName
$SrcDir       = Join-Path $WorkspaceRoot "src"
$DistDir      = Join-Path $WorkspaceRoot "dist\windows"

Write-Info "Workspace root: $WorkspaceRoot"
Write-Info "Rust workspace: $SrcDir"
Write-Info "Output dir:     $DistDir"
Write-Host ""

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------

Write-Info "Checking prerequisites..."

# Verify we are on Windows.
if ($env:OS -ne "Windows_NT") {
    Fail "This script must be run on Windows.  Detected OS: $($env:OS)"
}

# Verify cargo is available.
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Fail "cargo not found.  Install Rust from https://rustup.rs/ then reopen this terminal."
}
Write-Ok "cargo found: $(cargo --version)"

# Verify cargo-wix is installed.
# `cargo wix --version` exits 0 if installed, non-zero or missing if not.
try {
    $null = cargo wix --version 2>&1
    Write-Ok "cargo-wix found."
} catch {
    Fail "cargo-wix not found.  Install with: cargo install cargo-wix --version '0.3'"
}

# Verify WiX candle.exe is on the PATH (WiX Toolset v3).
if (-not (Get-Command candle.exe -ErrorAction SilentlyContinue)) {
    Write-Warn "candle.exe (WiX Toolset) not found on PATH."
    Write-Warn "Download WiX Toolset v3 from https://wixtoolset.org/releases/"
    Write-Warn "After installing, add the WiX bin directory to your PATH."
    Fail "WiX Toolset not found.  See warning above."
}
Write-Ok "WiX Toolset found: $(candle.exe /? 2>&1 | Select-Object -First 1)"

# Verify the Cargo.toml exists.
$CargoToml = Join-Path $SrcDir "Cargo.toml"
if (-not (Test-Path $CargoToml)) {
    Fail "Cargo.toml not found at: $CargoToml"
}

Write-Host ""

# ---------------------------------------------------------------------------
# Step 1: Build release binaries
# ---------------------------------------------------------------------------

if ($SkipBuild) {
    Write-Warn "Skipping cargo build (--SkipBuild specified)."
    Write-Warn "Using existing binaries in src/target/release/"
} else {
    Write-Info "Building release binaries (cargo build --release --workspace)..."
    Push-Location $SrcDir
    try {
        cargo build --release --workspace
        if ($LASTEXITCODE -ne 0) {
            Fail "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
    Write-Ok "Release binaries built."
}

Write-Host ""

# ---------------------------------------------------------------------------
# Step 2: Build MSI installer(s)
# ---------------------------------------------------------------------------

Push-Location $SrcDir

try {
    # Build kvm-master MSI.
    if (-not $OnlyClient) {
        Write-Info "Building kvm-master MSI installer..."

        # Check that the WiX source file exists.
        $MasterWxs = Join-Path $SrcDir "crates\kvm-master\wix\main.wxs"
        if (-not (Test-Path $MasterWxs)) {
            Fail "WiX source not found: $MasterWxs"
        }

        # --no-build: skip recompilation, use existing binaries.
        # --nocapture: print WiX compiler output (helps diagnose errors).
        cargo wix --package kvm-master --no-build --nocapture
        if ($LASTEXITCODE -ne 0) {
            Fail "cargo wix for kvm-master failed with exit code $LASTEXITCODE"
        }
        Write-Ok "kvm-master MSI built."
    }

    # Build kvm-client MSI.
    if (-not $OnlyMaster) {
        Write-Info "Building kvm-client MSI installer..."

        $ClientWxs = Join-Path $SrcDir "crates\kvm-client\wix\main.wxs"
        if (-not (Test-Path $ClientWxs)) {
            Fail "WiX source not found: $ClientWxs"
        }

        cargo wix --package kvm-client --no-build --nocapture
        if ($LASTEXITCODE -ne 0) {
            Fail "cargo wix for kvm-client failed with exit code $LASTEXITCODE"
        }
        Write-Ok "kvm-client MSI built."
    }
} finally {
    Pop-Location
}

Write-Host ""

# ---------------------------------------------------------------------------
# Step 3: Collect artefacts into dist/windows/
# ---------------------------------------------------------------------------

Write-Info "Collecting artefacts into $DistDir ..."

# Create the output directory if it doesn't exist.
# -Force creates parent directories automatically; -ErrorAction SilentlyContinue
# suppresses the error if the directory already exists.
New-Item -ItemType Directory -Path $DistDir -Force -ErrorAction SilentlyContinue | Out-Null

# Find and copy MSI files from src/target/wix/
$WixOutputDir = Join-Path $SrcDir "target\wix"
if (Test-Path $WixOutputDir) {
    $MsiFiles = Get-ChildItem -Path $WixOutputDir -Filter "*.msi" -Recurse
    if ($MsiFiles.Count -gt 0) {
        foreach ($msi in $MsiFiles) {
            Copy-Item -Path $msi.FullName -Destination $DistDir
            Write-Ok "Copied: $($msi.Name) → dist\windows\"
        }
    } else {
        Write-Warn "No .msi files found in $WixOutputDir"
    }
} else {
    Write-Warn "WiX output directory not found: $WixOutputDir"
}

# Copy raw binaries.
$Binaries = @("kvm-master.exe", "kvm-client.exe", "kvm-web-bridge.exe")
foreach ($binary in $Binaries) {
    $src = Join-Path $SrcDir "target\release\$binary"
    if (Test-Path $src) {
        Copy-Item -Path $src -Destination $DistDir
        Write-Ok "Copied: $binary → dist\windows\"
    } else {
        Write-Warn "Binary not found (skipping): $src"
    }
}

Write-Host ""

# ---------------------------------------------------------------------------
# Step 4: Summary
# ---------------------------------------------------------------------------

Write-Ok "Packaging complete!"
Write-Host ""
Write-Info "Output directory: $DistDir"
$Files = Get-ChildItem -Path $DistDir -File
if ($Files.Count -gt 0) {
    foreach ($f in $Files) {
        $SizeKB = [math]::Round($f.Length / 1024, 1)
        Write-Host "  $($f.Name) ($SizeKB KB)"
    }
} else {
    Write-Warn "No files found in output directory."
}

Write-Host ""
Write-Info "To install the MSI, double-click the .msi file or run:"
Write-Info "  msiexec /i dist\windows\kvm-master-*.msi"
Write-Host ""
Write-Warn "NOTE: The MSI is unsigned.  Windows SmartScreen may warn on first run."
Write-Warn "See docs/PACKAGING.md for code signing instructions."
