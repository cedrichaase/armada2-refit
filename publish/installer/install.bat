@echo off
rem Armada II Refit -- runs install.ps1; the options are the same (install.bat -Uninstall ...).
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install.ps1" %*
if errorlevel 1 (echo. & echo Install failed -- see above.)
pause
