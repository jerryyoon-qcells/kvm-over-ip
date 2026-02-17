# Build Windows MSI installers for kvm-master and kvm-client
# Requires: cargo-wix (cargo install cargo-wix), WiX v4 (dotnet tool install --global wix)
# Run from project root: powershell -ExecutionPolicy Bypass -File build\build_msi.ps1

$ErrorActionPreference = 'Stop'

$xwinSdk = "C:\Users\jerry\AppData\Local\Temp\xwin_sdk"
$msvcBin = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64"
$env:LIB  = "$xwinSdk\sdk\lib\um\x86_64;$xwinSdk\sdk\lib\ucrt\x86_64;$xwinSdk\crt\lib\x86_64;" +
             "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\lib\x64"
$env:PATH = "$msvcBin;$env:PATH"

$root   = "C:\Users\jerry\Projects\Claude_project"
$src    = "$root\src"
$outDir = "$root\dist"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

Write-Host "=== Building kvm-master MSI ===" -ForegroundColor Cyan
Set-Location $src
cargo wix --package kvm-master --nocapture --output "$outDir"
if ($LASTEXITCODE -ne 0) { Write-Error "kvm-master MSI failed"; exit 1 }

Write-Host "`n=== Building kvm-client MSI ===" -ForegroundColor Cyan
cargo wix --package kvm-client --nocapture --output "$outDir"
if ($LASTEXITCODE -ne 0) { Write-Error "kvm-client MSI failed"; exit 1 }

Write-Host "`n=== MSI packages in $outDir ===" -ForegroundColor Green
Get-ChildItem $outDir -Filter "*.msi" | Format-Table Name, Length, LastWriteTime
