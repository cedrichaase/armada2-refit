---
layers: [ui]
bump: minor
games: [fo-roots]
status: Installed, not yet seen in game
---

### Changed
- Fleet Operations: Roots: the briefing and objectives and the script's message boxes are the UI's (`[HUD] Messages=1` in `ui/defaults/fo-roots.ini`), so they are no longer stretched on a screen wider than 16:9; a reinstall moves an install still at the old Roots default 0 to 1 (`tools/ini-merge.py`, per game, with `A2_PROFILE=fo-roots`); a value set in the Refit screen (`RefitPlayer.ini`) still wins.

### Fixed
- The objectives key opens the briefing box of ours on Roots: the box's focus-step no longer runs on the pass the box opened, where Roots' toggle key closed it again at once.

### Added
- `UI.log` says when the engine counts the game paused and running again (`panels: the game paused` / `runs`).
