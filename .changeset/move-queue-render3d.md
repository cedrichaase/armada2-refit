---
layers: [render3d]
bump: minor
status: Confirmed in game 2026-11-06
---

### Added
- `[Overlays] MovePointColour`, `MovePointBrightness`, `MovePointOpacity`.

### Changed
- Overlays: move paths are drawn for the selected ships only (the overview's `GetSelectList`, so a selection above 16 is read too; entries `ov_vt_select_num`, `ov_vt_select_list` in `profiles/gog-1.1/render3d.overlays.toml`).
- Overlays: the path is QOL.asi's queue when it exports one (`QOL_MoveQueueVersion`, `QOL_MoveQueue`: appended legs, a patrol run closed back to its first point), found by its export (any loaded module, not by file name) once a second until found; no sketch is drawn then. Without it, the engine's path (the "ends from a sketch" filter) and the sketch as before, for selected ships.
- Overlays: the line is finer (`MovePathDot` 2.5 -> 1.9, `MovePathSpacing` 9 -> 8) and very light grey (`MovePathColour` 0.85 0.85 0.85); the points keep their size and colour, now their own keys.
- Install: an installed `Render3D.ini` still holding the old line defaults (`MovePathColour=0.55 1.0 0.55`, `MovePathDot=2.5`, `MovePathSpacing=9`) gets the new ones (`tools/ini-merge.py` RETIRED); a value the player set stays.
- Render3D.log: `move paths: N selected ship(s) on a path, M point(s) ahead (qol|engine|-), sketch K`, and `QOL's queue found (version 1)` / `not found`.
