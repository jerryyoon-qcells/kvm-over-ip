# Build and test Rust workspace using xwin SDK stubs
param(
    [string]$CargoArgs = "build --workspace"
)

$xwinSdk = "C:\Users\jerry\AppData\Local\Temp\xwin_sdk"
$msvcBin = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64"
$cargoExe = "C:\Users\jerry\.cargo\bin\cargo.exe"
$workspaceDir = "C:\Users\jerry\Projects\Claude_project\src"

# Set up LIB paths (semicolon-separated for MSVC linker)
$libPaths = @(
    "$xwinSdk\sdk\lib\um\x86_64",
    "$xwinSdk\sdk\lib\ucrt\x86_64",
    "$xwinSdk\crt\lib\x86_64",
    "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\lib\x64"
)
$libEnv = $libPaths -join ";"
Write-Output "LIB=$libEnv"

# Set PATH to include MSVC bin FIRST
$env:PATH = "$msvcBin;$($env:PATH)"
$env:LIB = $libEnv

# Also set INCLUDE for headers (needed for some crates)
$xwinInclude = "$xwinSdk\sdk\include"
$msvcInclude = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\include"
$env:INCLUDE = "$msvcInclude;$xwinInclude\10.0.26100\ucrt;$xwinInclude\10.0.26100\shared;$xwinInclude\10.0.26100\um;$xwinInclude\10.0.26100\winrt"

Write-Output "Running cargo $CargoArgs..."
Write-Output "With MSVC linker from: $msvcBin"
Write-Output ""

Set-Location $workspaceDir
$result = & $cargoExe $CargoArgs.Split(' ') 2>&1
$result | Write-Output
$exitCode = $LASTEXITCODE
Write-Output "`nExit code: $exitCode"
exit $exitCode
