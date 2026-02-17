@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" > NUL 2>&1
cd /d "C:\Users\jerry\Projects\Claude_project\src"
cargo build -p kvm-web-bridge > "C:\Users\jerry\Projects\Claude_project\build_output.txt" 2>&1
echo EXIT_CODE=%ERRORLEVEL% >> "C:\Users\jerry\Projects\Claude_project\build_output.txt"
exit /b %ERRORLEVEL%
