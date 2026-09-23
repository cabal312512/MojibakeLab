@echo off
setlocal
cd /d "%~dp0"
if not exist "release\MojibakeLab\MojibakeLab.exe" (
  echo Build the application first: powershell -ExecutionPolicy Bypass -File scripts\build.ps1
  pause
  exit /b 1
)
set "MOJIBAKE_DATA_DIR=%~dp0runtime-data"
if not exist "%MOJIBAKE_DATA_DIR%\temp" mkdir "%MOJIBAKE_DATA_DIR%\temp"
set "TEMP=%MOJIBAKE_DATA_DIR%\temp"
set "TMP=%TEMP%"
start "" "release\MojibakeLab\MojibakeLab.exe"
