---
layers: [root]
bump: patch
games: [armada2, fo-roots]
---

### Fixed
- retire: installing (`./install`, `install.sh`, `install.ps1`) takes out the plugin logs older builds wrote beside the exe (`Display8.log`, `Render3D.log`, `UI.log`, `QOL.log`, `QOLRules.log`, `Online.log`, `BinkProxy.log`; `file` entries in `retire/known.txt`). Since 13.50.0 the logs are in `%LOCALAPPDATA%\armada2-refit\logs`; a stale one in the game folder was sent with a crash report as the current one (#87).
