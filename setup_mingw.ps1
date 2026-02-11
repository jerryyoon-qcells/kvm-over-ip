# Download and install portable MinGW-w64 (winlibs)
# This is a portable GCC that can build Windows applications without MSVC SDK

$destDir = "$env:USERPROFILE\mingw64"

if (Test-Path "$destDir\bin\gcc.exe") {
    Write-Output "MinGW-w64 already installed at $destDir"
    exit 0
}

Write-Output "Downloading portable MinGW-w64 (winlibs)..."
# winlibs.com provides portable MinGW-w64 builds
$url = "https://github.com/brechtsanders/winlibs_mingw/releases/download/14.2.0posix-19.1.7-12.0.0-ucrt-r2/winlibs-x86_64-posix-seh-gcc-14.2.0-mingw-w64ucrt-12.0.0-r2.7z"
$archive = "$env:TEMP\mingw64.7z"

try {
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12
    $client = New-Object System.Net.WebClient
    $client.DownloadFile($url, $archive)
    Write-Output "Download complete: $archive"
    Write-Output "File size: $((Get-Item $archive).Length / 1MB) MB"
} catch {
    Write-Output "Download failed: $_"
    exit 1
}
