# Try to get minimal Windows SDK stubs via NuGet packages
# Microsoft publishes the Windows SDK as NuGet packages

$nugetDir = "$env:TEMP\winsdk_nuget"
New-Item -ItemType Directory -Force -Path $nugetDir | Out-Null

# SDK 10.0.22621.0 x64 um (User Mode) library stubs
$sdkPkgs = @(
    "https://globalcdn.nuget.org/packages/microsoft.windows.sdk.contracts.10.0.22621.2428.nupkg"
)

# Actually, let's try the direct approach using xwin from GitHub
$xwinReleases = Invoke-RestMethod -Uri "https://api.github.com/repos/Jake-Shadle/xwin/releases/latest" -UseBasicParsing 2>&1
Write-Output "Latest xwin release: $($xwinReleases.tag_name)"

$windowsAsset = $xwinReleases.assets | Where-Object { $_.name -like "*x86_64*windows*" } | Select-Object -First 1
if ($windowsAsset) {
    Write-Output "Asset: $($windowsAsset.name) - $($windowsAsset.browser_download_url)"
    $dest = "$env:TEMP\xwin.zip"
    Invoke-WebRequest -Uri $windowsAsset.browser_download_url -OutFile $dest -UseBasicParsing
    Write-Output "Downloaded xwin: $((Get-Item $dest).Length / 1KB) KB"
    Expand-Archive -Path $dest -DestinationPath "$env:TEMP\xwin_bin" -Force
    $xwinExe = Get-ChildItem "$env:TEMP\xwin_bin" -Filter "xwin.exe" -Recurse | Select-Object -First 1
    if ($xwinExe) {
        Write-Output "xwin.exe found at: $($xwinExe.FullName)"
        # Use xwin to download and install the Windows SDK headers/libs
        & $xwinExe.FullName --accept-license --output "$env:TEMP\xwin_sdk" splat --include-debug-libs --include-debug-symbols 2>&1
        Write-Output "xwin completed"
    }
} else {
    Write-Output "No Windows asset found"
}
