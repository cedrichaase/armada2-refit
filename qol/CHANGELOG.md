# Changelog — qol

`QOL.asi`: gameplay quality of life that stays compatible with stock players, and the
plan for the rest. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. Details, and the planned changes, are in
[`README.md`](README.md).

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

Installed, not yet seen in game. On the bench: pay, start, refusal and cancel
(`qol-pay-on-queue`); destruction, capture and save/load not yet run.

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
