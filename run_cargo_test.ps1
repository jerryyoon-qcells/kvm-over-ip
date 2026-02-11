# Initialize MSVC environment and run cargo
# Use vsdevcmd.bat which sets up all required paths including Windows SDK

param(
    [string]$CargoArgs = "build --workspace"
)

# Find the developer command prompt batch file
$devCmdPaths = @(
    "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat",
    "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat",
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat"
)

$devCmd = $null
foreach ($path in $devCmdPaths) {
    if (Test-Path $path) {
        $devCmd = $path
        break
    }
}

if (-not $devCmd) {
    Write-Error "Could not find VsDevCmd.bat"
    exit 1
}

Write-Output "Using: $devCmd"

# Run a cmd shell that first sources vcvars, then runs cargo
$cargoExe = "C:\Users\jerry\.cargo\bin\cargo.exe"
$workspaceDir = "C:\Users\jerry\Projects\Claude_project\src"

$cmdScript = @"
call "$devCmd" -arch=x64 -no_logo
echo LIB=%LIB%
echo INCLUDE=%INCLUDE%
echo WindowsSdkDir=%WindowsSdkDir%
echo WindowsSdkVersion=%WindowsSdkVersion%
cd "$workspaceDir"
"$cargoExe" $CargoArgs
"@

$tmpBat = [System.IO.Path]::GetTempFileName() + ".bat"
Set-Content -Path $tmpBat -Value $cmdScript

Write-Output "Running: $cmdScript"
Write-Output "---"

$proc = Start-Process -FilePath "cmd.exe" -ArgumentList "/c `"$tmpBat`"" -Wait -PassThru -NoNewWindow
Write-Output "Exit code: $($proc.ExitCode)"

Remove-Item $tmpBat -ErrorAction SilentlyContinue
