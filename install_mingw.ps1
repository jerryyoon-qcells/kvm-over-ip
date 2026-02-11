# Install LLVM/Clang which can act as a linker for MSVC target (lld-link)
# This approach doesn't require Windows SDK

# First check if winget can install LLVM without admin
$result = & winget install --id LLVM.LLVM --accept-package-agreements --accept-source-agreements --scope user 2>&1
Write-Output "LLVM install result: $($result | Select-Object -Last 3)"

# Check if lld-link exists after install
$lldPath = "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\LLVM.LLVM*\bin\lld-link.exe"
$found = Get-ChildItem $lldPath -ErrorAction SilentlyContinue | Select-Object -First 1
if ($found) {
    Write-Output "lld-link found at: $($found.FullName)"
} else {
    Write-Output "lld-link not found via winget path"
    # Try standard LLVM install paths
    $paths = @("C:\Program Files\LLVM\bin\lld-link.exe", "C:\Program Files (x86)\LLVM\bin\lld-link.exe")
    foreach ($p in $paths) {
        if (Test-Path $p) { Write-Output "Found: $p" }
    }
}
