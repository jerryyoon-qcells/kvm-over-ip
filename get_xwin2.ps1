# Download xwin and use it to get Windows SDK stubs
$xwinTar = "C:\Users\jerry\AppData\Local\Temp\xwin.tar.gz"
$xwinDir = "C:\Users\jerry\AppData\Local\Temp\xwin_extracted"
$xwinSdk = "C:\Users\jerry\AppData\Local\Temp\xwin_sdk"

# Check if already done
if (Test-Path "$xwinSdk\sdk\lib\um\x86_64\kernel32.lib") {
    Write-Output "Windows SDK stubs already available at $xwinSdk"
    exit 0
}

if (-not (Test-Path $xwinTar)) {
    Write-Output "Downloading xwin..."
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri "https://github.com/Jake-Shadle/xwin/releases/download/0.8.0/xwin-0.8.0-x86_64-pc-windows-msvc.tar.gz" -OutFile $xwinTar -UseBasicParsing
    Write-Output "Downloaded: $([math]::Round((Get-Item $xwinTar).Length / 1KB, 1)) KB"
}

# Extract using Windows tar.exe (cmd.exe compatible path)
if (-not (Test-Path $xwinDir)) {
    New-Item -ItemType Directory -Force -Path $xwinDir | Out-Null
}

Write-Output "Extracting with Windows tar..."
# Use cmd.exe tar which handles Windows paths properly
$tarExe = "C:\Windows\System32\tar.exe"
if (Test-Path $tarExe) {
    & $tarExe -xzf $xwinTar -C $xwinDir 2>&1
    Write-Output "Extraction complete"
} else {
    Write-Output "Windows tar.exe not found"
    exit 1
}

$xwinExe = Get-ChildItem $xwinDir -Filter "xwin.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $xwinExe) {
    Write-Output "xwin.exe not found. Contents of $xwinDir :"
    Get-ChildItem $xwinDir -Recurse | Select-Object FullName | Select-Object -First 20
    exit 1
}

Write-Output "xwin.exe found: $($xwinExe.FullName)"
Write-Output "Downloading Windows SDK via xwin..."

New-Item -ItemType Directory -Force -Path $xwinSdk | Out-Null

$result = & $xwinExe.FullName --accept-license --output $xwinSdk splat --include-debug-libs 2>&1
Write-Output "xwin output:"
$result | Select-Object -First 20

if (Test-Path $xwinSdk) {
    Write-Output "SDK directories:"
    Get-ChildItem $xwinSdk -Recurse | Where-Object { -not $_.PSIsContainer } |
        Where-Object { $_.Name -like "kernel32*" } | Select-Object FullName
}
