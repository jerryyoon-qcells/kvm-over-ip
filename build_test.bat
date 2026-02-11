@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" > nul 2>&1
echo LIB=%LIB%
echo PATH_CHECK=
where link.exe 2>&1
echo VCINSTALLDIR=%VCINSTALLDIR%
echo WindowsSdkDir=%WindowsSdkDir%
echo WindowsSdkVersion=%WindowsSdkVersion%
