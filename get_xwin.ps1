# Download xwin and use it to get Windows SDK stubs
$xwinTar = "$env:TEMP\xwin.tar.gz"
$xwinDir = "$env:TEMP\xwin_extracted"
$xwinSdk = "$env:TEMP\xwin_sdk"

# Check if already done
if (Test-Path "$xwinSdk\crt\lib\x86_64\kernel32.lib") {
    Write-Output "Windows SDK stubs already available"
    exit 0
}

if (-not (Test-Path $xwinTar)) {
    Write-Output "Downloading xwin..."
    Invoke-WebRequest -Uri "https://github.com/Jake-Shadle/xwin/releases/download/0.8.0/xwin-0.8.0-x86_64-pc-windows-msvc.tar.gz" -OutFile $xwinTar -UseBasicParsing
    Write-Output "Downloaded: $((Get-Item $xwinTar).Length / 1KB) KB"
}

# Extract using tar (available on Windows 10 1803+)
New-Item -ItemType Directory -Force -Path $xwinDir | Out-Null
$tarResult = & tar -xzf $xwinTar -C $xwinDir 2>&1
Write-Output "Extraction: $tarResult"

$xwinExe = Get-ChildItem $xwinDir -Filter "xwin.exe" -Recurse | Select-Object -First 1
if (-not $xwinExe) {
    Write-Output "xwin.exe not found after extraction"
    Get-ChildItem $xwinDir -Recurse | Select-Object Name | Select-Object -First 10
    exit 1
}

Write-Output "xwin.exe found: $($xwinExe.FullName)"

# Use xwin to download and splat the Windows SDK
New-Item -ItemType Directory -Force -Path $xwinSdk | Out-Null
Write-Output "Downloading Windows SDK via xwin (this may take a few minutes)..."

$xwinArgs = "--accept-license --output `"$xwinSdk`" splat --output `"$xwinSdk`" --include-debug-libs"
$result = & $xwinExe.FullName --accept-license --output "$xwinSdk" splat --include-debug-libs 2>&1
Write-Output "xwin result: $result"

# Check results
if (Test-Path "$xwinSdk") {
    Write-Output "SDK directories created:"
    Get-ChildItem "$xwinSdk" | Select-Object Name
}
