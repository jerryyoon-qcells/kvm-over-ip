$xwinExe = "C:\Users\jerry\AppData\Local\Temp\xwin_extracted\xwin-0.8.0-x86_64-pc-windows-msvc\xwin.exe"
$xwinSdk = "C:\Users\jerry\AppData\Local\Temp\xwin_sdk"

New-Item -ItemType Directory -Force -Path $xwinSdk | Out-Null

Write-Output "Running xwin to download Windows SDK stubs..."
Write-Output "This downloads from Microsoft's servers and may take a few minutes."

# xwin 0.8.0 syntax: xwin --accept-license splat --output <dir>
$result = & $xwinExe --accept-license splat --output $xwinSdk 2>&1
Write-Output "xwin output:"
$result

if (Test-Path $xwinSdk) {
    Write-Output "`nSDK directory contents:"
    Get-ChildItem $xwinSdk -Recurse | Where-Object { $_.Name -like "kernel32*" } | Select-Object FullName
    Get-ChildItem $xwinSdk | Select-Object Name
}
