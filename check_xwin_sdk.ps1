$xwinSdk = "C:\Users\jerry\AppData\Local\Temp\xwin_sdk"

Write-Output "=== xwin SDK contents ==="
if (Test-Path "$xwinSdk\sdk\lib\um\x86_64") {
    Write-Output "SDK UM x86_64 libs:"
    Get-ChildItem "$xwinSdk\sdk\lib\um\x86_64" | Select-Object Name | Format-Wide -Column 4
}
if (Test-Path "$xwinSdk\crt\lib\x86_64") {
    Write-Output "`nCRT x86_64 libs:"
    Get-ChildItem "$xwinSdk\crt\lib\x86_64" | Select-Object Name | Format-Wide -Column 4
}

Write-Output "`n=== Checking key files ==="
$keyFiles = @(
    "$xwinSdk\sdk\lib\um\x86_64\kernel32.Lib",
    "$xwinSdk\sdk\lib\um\x86_64\ntdll.Lib",
    "$xwinSdk\sdk\lib\um\x86_64\ws2_32.Lib",
    "$xwinSdk\sdk\lib\um\x86_64\userenv.Lib",
    "$xwinSdk\sdk\lib\um\x86_64\dbghelp.Lib",
    "$xwinSdk\crt\lib\x86_64\msvcrt.Lib",
    "$xwinSdk\crt\lib\x86_64\libcmt.Lib"
)
foreach ($f in $keyFiles) {
    $exists = Test-Path $f
    Write-Output "$exists : $f"
}
