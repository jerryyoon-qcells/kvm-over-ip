param([string]$CargoArgs = "build --workspace")

$xwinSdk = "C:\Users\jerry\AppData\Local\Temp\xwin_sdk"
$msvcBin = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64"
$cargoExe = "C:\Users\jerry\.cargo\bin\cargo.exe"
$workspaceDir = "C:\Users\jerry\Projects\Claude_project\src"

$libPaths = @(
    "$xwinSdk\sdk\lib\um\x86_64",
    "$xwinSdk\sdk\lib\ucrt\x86_64",
    "$xwinSdk\crt\lib\x86_64",
    "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\lib\x64"
)
$env:LIB = $libPaths -join ";"
$env:PATH = "$msvcBin;$($env:PATH)"

Set-Location $workspaceDir
& $cargoExe $CargoArgs.Split(' ')
exit $LASTEXITCODE
