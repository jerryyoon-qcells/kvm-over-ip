# Build all Windows packages: install cargo-wix, then build MSI installers
# Run from the project root: powershell -ExecutionPolicy Bypass -File build\install_and_package.ps1

$ErrorActionPreference = 'Stop'

# --- SDK environment setup (same as cargo_build_full.ps1) ---
$xwinSdk  = "C:\Users\jerry\AppData\Local\Temp\xwin_sdk"
$msvcBin  = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64"

$env:LIB = "$xwinSdk\sdk\lib\um\x86_64;$xwinSdk\sdk\lib\ucrt\x86_64;$xwinSdk\crt\lib\x86_64;" +
           "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\lib\x64"
$env:PATH = "$msvcBin;$env:PATH"

$root = "C:\Users\jerry\Projects\Claude_project"
$src  = "$root\src"

Write-Host "=== Step 1: Install cargo-wix ===" -ForegroundColor Cyan
cargo install cargo-wix
if ($LASTEXITCODE -ne 0) { Write-Error "cargo-wix install failed"; exit 1 }

Write-Host "`n=== Step 2: Build release binaries ===" -ForegroundColor Cyan
Set-Location $src
cargo build --workspace --release
if ($LASTEXITCODE -ne 0) { Write-Error "Release build failed"; exit 1 }

# Create output directory for packages
$outDir = "$root\dist"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

Write-Host "`n=== Step 3: Build kvm-master MSI ===" -ForegroundColor Cyan
Set-Location "$src\crates\kvm-master"
cargo wix --nocapture --output "$outDir"
if ($LASTEXITCODE -ne 0) { Write-Warning "kvm-master MSI build failed (WiX Toolset may not be installed)" }

Write-Host "`n=== Step 4: Build kvm-client MSI ===" -ForegroundColor Cyan
Set-Location "$src\crates\kvm-client"
cargo wix --nocapture --output "$outDir"
if ($LASTEXITCODE -ne 0) { Write-Warning "kvm-client MSI build failed (WiX Toolset may not be installed)" }

Write-Host "`n=== Step 5: Copy raw binaries to dist ===" -ForegroundColor Cyan
Copy-Item "$src\target\release\kvm-master.exe"     $outDir -Force
Copy-Item "$src\target\release\kvm-client.exe"     $outDir -Force
Copy-Item "$src\target\release\kvm-web-bridge.exe" $outDir -Force

Write-Host "`n=== Build complete. Artifacts in $outDir ===" -ForegroundColor Green
Get-ChildItem $outDir | Format-Table Name, Length, LastWriteTime
