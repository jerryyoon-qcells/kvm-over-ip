$cargoExe = 'C:\Users\jerry\.cargo\bin\cargo.exe'
Write-Output "Attempting to install cargo-xwin..."
# Note: cargo-xwin itself needs compilation - which will have the same linker issue
# Instead, let's try installing it pre-built from GitHub releases

$xwinUrl = "https://github.com/rust-cross/cargo-xwin/releases/latest/download/cargo-xwin-x86_64-pc-windows-msvc.zip"
$destDir = "C:\Users\jerry\.cargo\bin"
$archivePath = "$env:TEMP\cargo-xwin.zip"

try {
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12
    Write-Output "Downloading cargo-xwin..."
    Invoke-WebRequest -Uri $xwinUrl -OutFile $archivePath -UseBasicParsing
    Write-Output "Downloaded: $((Get-Item $archivePath).Length / 1KB) KB"

    Expand-Archive -Path $archivePath -DestinationPath $env:TEMP\cargo-xwin-extracted -Force
    Get-ChildItem "$env:TEMP\cargo-xwin-extracted" -Recurse | Where-Object { $_.Name -like "cargo-xwin*" }

} catch {
    Write-Output "Failed: $_"
}
