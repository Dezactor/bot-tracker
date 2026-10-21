@echo off
chcp 65001 >nul
setlocal

set "SRC=%~dp0"
set "TMP=%TEMP%\bot-tracker-rust-build"

if exist "%TMP%" rmdir /s /q "%TMP%"
mkdir "%TMP%"

robocopy "%SRC%" "%TMP%" /E /XD target /NFL /NDL /NJH /NJS >nul

pushd "%TMP%"
set "PATH=C:\mingw64\mingw64\bin;%PATH%"
cargo build --release
if errorlevel 1 (
    popd
    echo Build failed
    exit /b 1
)
copy /y "target\release\bot-tracker.exe" "%SRC%bot-tracker.exe" >nul
popd

echo Done: %SRC%bot-tracker.exe
endlocal
