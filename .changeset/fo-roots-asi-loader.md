---
asiloader: minor
render3d: patch
root: patch
status: Installed, not yet seen in game
---

### Added
- The `asiloader` layer: the Ultimate ASI Loader as `winmm.dll` in the executable's folder of a game that ships none (Fleet Operations: Roots), after Display8 and with it; a `winmm.dll` that is not ours is kept as `winmm.dll.a2asi-backup`; a2mod files it by content. Refused on Armada II.

### Changed
- render3d: `install.sh` installs into the executable's folder (`Data/` on Fleet Operations: Roots; the game directory on Armada II, as before) and runs `retire/retire.sh` on Armada II only; render3d is a layer of Roots.
- a2mod and `tools/layers.py` file added files, replaced files and backups only under the layers of the chosen game; `--check` asserts `winmm.dll` is no layer's on Armada II.

### Fixed
- asiloader: with a backup already present and a different foreign `winmm.dll` in place, `install` refuses with a message instead of overwriting.
