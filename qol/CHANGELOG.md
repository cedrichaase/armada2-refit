# Changelog — qol

`QOL.asi`: gameplay quality of life that stays compatible with stock players, and the
plan for the rest. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. Details, and the planned changes, are in
[`README.md`](README.md).

## 1.7.1 — 2026-10-30

### Changed
- `qol.c` and `rules.c` shed their own copies of the helpers; the `.def` files moved to `csdk/def/`. Built on `csdk/a2asi.h` (the Win32 types, kernel32 imports, `s_*` string helpers, the append-a-line log, signature check and `VirtualProtect` patch) with the import libraries in `csdk/def/`; behaviour and log lines are unchanged.

Installed, not yet seen in game. (HASH)

## 1.6.4 — 2026-10-23

### Fixed
- `build.sh` takes a lock (`build/.build.lock`), so two builds of one tree at once
  (`a2test -j` installing one checkout twice) no longer write the same files in `build/`.

Installed, not yet seen in game.

## 1.6.3 — 2026-10-21

### Changed
- `QOL.asi` and `QOLRules.asi` each carry a version resource (`VERSIONINFO`): product,
  description, the layer's version as the file version and the project's as the
  product version, which Windows shows in the file's Properties. Built by `platform/versioninfo/build-res.sh` (915cad8).

Confirmed in game 2026-10-22.

## 1.6.2 — 2026-10-21

### Fixed
- `QOLRules.asi`: an order refused under `PayOnQueue=1` was dropped in silence; it now
  speaks stock's advisor line for the first resource short (dilithium, officers, crew,
  latinum, metal, biomatter, in `StartBuild`'s order), to the refused player only, with
  stock's cadence, and logs an `advisor:` line (issue #1). Scenario `qol-refused-advisor` (c23cb19).

Confirmed in game 2026-10-22.

## 1.6.1 — 2026-10-12

### Fixed
- `QOLRules.asi`: a destroyed producer gave its item in progress back twice (DestroyShip,
  then ClearTeam when the object is cleaned up), and a craft that cannot die refunded its
  queue on every fatal hit. DestroyShip is no longer hooked; ClearTeam gives it all back,
  once.
- `QOLRules.asi`: losing a producer (destroyed, captured, assimilated) returned the
  officers of the item in progress twice; they are left to stock's
  `mRemoveConstructionEffect` when it returns them.
- `QOLRules.asi`: the network game's answer (`QOLRules_Network`) was latched once per
  session; it is now cleared at every game start (`Program::GameOpenPreLoad`), so a second
  game in the same session cannot run on a stale answer.
- `QOLRules.asi`: a delete order sent while an item waited was lost if the item started
  before it arrived; ids are matched with the paid mark masked.
- `QOL.ini`: a number with more than nine digits before the point overflowed and parsed
  as garbage; it is now refused like any malformed value.

Installed, not yet seen in game.

## 1.6.0 — 2026-10-07

### Added
- `QOLRules.asi` exports `QOLRules_Wanted`, `QOLRules_Network` and `QOLRules_Reset`; in a
  network game it applies its rules when `Online.asi` reports that every player runs it
  (latched at first use in a game), instead of standing down always.

Confirmed in game 2026-10-09.

## 1.5.0 — 2026-10-07

### Added
- `QOLRules.asi` / `QOLRules.ini` (`PayOnQueue=`, default 1; `Log=`): QOL-7. An item is
  paid when queued, not when it starts; an order the bank cannot cover (resources, crew,
  officers) is dropped; cancelling or deleting a queued item, and losing the building
  (destroyed, captured, assimilated), gives its cost back in full, crew included. Saves keep
  which items are paid. Applies in single player and skirmish only and stands down in
  network games (a rules change every peer must share). `a2mod` switches it with `qol`.

Confirmed in game 2026-10-09: pay on queue and the refund on cancel, single player.
Losing the building (destroyed, captured, assimilated) and save/load not yet seen in game
or run on the bench.

## 1.4.0 — 2026-10-07

### Added
- `ViewDistance=` (default 1000000; 0 leaves the view stock): `FAR_CLIPPING_PLANE` and
  `cfgOBJECT_CULLING_DISTANCE` (`ART_CFG.h`, stock 20000 and 2800) are raised to at least
  that after the parse, so nothing fades out with distance; fog and shroud still hide
  what they hid (QOL-9).

Confirmed in game 2026-10-07.

## 1.3.0 — 2026-10-07

### Added
- `WarpDistance=` (default 1100; 0 leaves moves stock): a move ordered on the map goes
  to warp, as the minimap's does, when the selected ships are on average farther than
  that from the point; the cursor shows the warp ring there. Only ships that can warp
  count, and nothing changes in a game with warp turned off. Alt+click still warps at
  any distance. The order is the minimap's `GO_WARP`, so stock players need nothing
  (QOL-8).

Confirmed in game 2026-10-07.

## 1.2.0 — 2026-10-05

### Added
- `StationGroups=` (default 1): this player's stations of one kind can be selected
  together (click, Shift-click, double click) and kept in one control group; recall
  selects them all, and its second press (or `Alt+number`) centres the camera on them.
  A group holds ships or one kind of station: `Shift+number` refuses anything else.
  With several stations selected the build menu is available, and each build order
  goes to the one with the shortest queue (in turn on a tie), each cancel to the
  longest (QOL-5, QOL-6). (5a5f2d0)

### Fixed
- `Ctrl+number` now replaces a group that held stations: before, the stations stayed
  in it with their group number drawn, and `Ctrl+number` on a station added to the
  group. (5a5f2d0)
- A ship built at a station in a control group no longer joins that group (stock gives
  a producer's builds its group); it put the ship in the ships' group, which recall
  prefers, so the key selected the new ships instead of the stations. (403dfad)

Confirmed in game 2026-10-05.

## 1.1.1 — 2026-10-05

### Fixed
- Selecting more than 16 ships with a special weapon (Galaxy, Vor'cha) crashed the game
  (R6025) or froze its frame loop: the button bar gathered them into a 16-entry array.
  `MaxSelection` over 16 now also moves that array, in setting up and in firing the
  special weapon, and is refused if it cannot.

Confirmed in game 2026-10-05.

## 1.1.0 — 2026-10-04

### Added
- `ShiftAddsToGroup=` (default 1): `Shift+number` adds the selection to that control
  group and keeps what is in it, as `Ctrl+Shift+number` does in stock. Stock's
  `Shift+number` (select the group and centre the camera on it) moves to `Alt+number`.
- `MaxSelection=` (default 40, 17–120; 16 or less leaves it stock): selections, and so
  control groups, hold more than stock's 16. The selection panel shows the first 16. (691256a)

Confirmed in game 2026-10-05.

## 1.0.0 — 2026-10-03

### Added
- `QOL.asi`, `QOL.ini`: right-click-drag pan speed, `PanSpeed=` (default 2.5, 0.25–10, 1
  leaves it alone). `RTS_CFG.h`'s `FASTSCROLL_COEFFICIENT` is scaled in memory as the
  game parses it, so the file stays stock and network games still accept the player
  (8a8248b).
- `install.sh` (`--pan-speed X`, `--remove`), and `rts-cfg-check.py`, which `install.sh`
  runs with `--fix` to put a hand-edited `FASTSCROLL_COEFFICIENT` back to stock.
- The ideas from the root `IDEAS.md` (QOL-1 to QOL-7) move to `README.md`, each marked
  for `QOL.asi` or for a future rules plugin that every player in a game needs.

Confirmed in game 2026-10-03.
