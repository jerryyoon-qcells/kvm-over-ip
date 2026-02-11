$vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$info = & $vswhere -all -products * -format json 2>&1
Write-Output $info

# Also check Windows SDK
$sdkReg = "HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots"
if (Test-Path $sdkReg) {
    $sdkProps = Get-ItemProperty $sdkReg
    Write-Output "SDK 10 Dir: $($sdkProps.'KitsRoot10')"
} else {
    Write-Output "Windows SDK registry key not found"
}
