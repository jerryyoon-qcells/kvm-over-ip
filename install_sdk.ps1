# Try to install Windows SDK via VS installer
$vsInstallerPath = "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vs_installer.exe"
if (Test-Path $vsInstallerPath) {
    Write-Output "VS Installer found, attempting to add Windows 11 SDK component..."
    # Run as quiet install adding Windows 11 SDK
    $proc = Start-Process -FilePath $vsInstallerPath `
        -ArgumentList "modify --installPath `"C:\Program Files\Microsoft Visual Studio\2022\Community`" --add Microsoft.VisualStudio.Component.Windows11SDK.22621 --quiet --norestart" `
        -Wait -PassThru -NoNewWindow
    Write-Output "Exit code: $($proc.ExitCode)"
} else {
    Write-Output "VS Installer not found at expected path"
}

# Check for installed SDK versions
$sdkReg = "HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots"
if (Test-Path $sdkReg) {
    $props = Get-ItemProperty $sdkReg
    $props | Format-List
}
