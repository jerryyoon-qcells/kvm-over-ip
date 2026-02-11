# Test if lld-link + clang-cl can build a simple program
$llvmBin = "C:\Program Files\LLVM\bin"
$clangCl = "$llvmBin\clang-cl.exe"
$lldLink = "$llvmBin\lld-link.exe"

$testDir = "$env:TEMP\lld_test"
New-Item -ItemType Directory -Force -Path $testDir | Out-Null

# Write a simple C source
Set-Content -Path "$testDir\test.c" -Value @"
#include <windows.h>
int main() { return 0; }
"@

Write-Output "Testing clang-cl + lld-link..."

# Try to compile
$compileResult = & $clangCl /c "$testDir\test.c" /Fo"$testDir\test.obj" 2>&1
Write-Output "Compile result: $compileResult"

if (Test-Path "$testDir\test.obj") {
    Write-Output "Compilation succeeded!"

    # Try to link
    $linkResult = & $lldLink "$testDir\test.obj" /out:"$testDir\test.exe" /defaultlib:msvcrt /entry:mainCRTStartup 2>&1
    Write-Output "Link result: $linkResult"

    if (Test-Path "$testDir\test.exe") {
        Write-Output "Linking succeeded! test.exe created."
    }
} else {
    Write-Output "Compilation failed, .obj not created"
}

# Cleanup
Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
