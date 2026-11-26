---
render3d: minor
ui: minor
games: [armada2]
status: Confirmed in game 2026-11-26
---

### Added
- The menu scene shows campaign progress (issue 15). render3d: `Campaign=`, `From=`, `Until=` on a `[Ship.*]` or `[Nebula.*]` (`Campaign=` also on a `[Group.*]`) put it on the map only at that many missions complete; `[Progress] Default=` and `[Progress] <campaign>=` in `map.ini`; the export `Render3D_MenuProgress(campaign, done)`, which re-reads the map on a change. The built-in map grows the Federation fleet (freighters, `fmining`, a second Sovereign and Akira), gates the Klingon cruisers and Negh'Var and the Borg Diamond by progress, and adds spheres to the Borg scene, over the ships issue #14 placed. UI: `menus::progress` tells render3d each campaign's missions complete before each scene frame, from the progress the campaign locks read (the game's memory, else `save\shell.set`). Draft: placements reckoned, not yet seen on the bench.
