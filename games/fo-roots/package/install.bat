@echo off
rem Armada II Refit for Fleet Operations: Roots: install.bat [-Uninstall] [<Roots folder>]
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install.ps1" %*
echo.
pause
