---
layers: [root]
bump: minor
games: [fo-roots]
---

### Added
- Fleet Operations: Roots: games/fo-roots/package/package.sh builds a private zip of the Roots layers (Display8, the ASI loader, Render3D, QOL and UI with Roots' defaults), prebuilt, with install.bat/install.ps1 for Windows and install.sh for Wine: backs up DxWrapper's d3d8.dll and a foreign winmm.dll, keeps the player's .ini files, -Uninstall/--uninstall puts everything back. Never a release; handed out by hand.
