# Changelog — qol

`QOL.asi`: gameplay quality of life that stays compatible with stock players, and the
plan for the rest. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. Details, and the planned changes, are in
[`README.md`](README.md).

## 1.1.1 — 2026-10-05

### Fixed
- Selecting more than 16 ships with a special weapon (Galaxy, Vor'cha) crashed the game
  (R6025) or froze its frame loop: the button bar gathered them into a 16-entry array.
  `MaxSelection` over 16 now also moves that array, in setting up and in firing the
  special weapon, and is refused if it cannot.

Installed, not yet seen in game.

## 1.1.0 — 2026-10-04

### Added
- `ShiftAddsToGroup=` (default 1): `Shift+number` adds the selection to that control
  group and keeps what is in it, as `Ctrl+Shift+number` does in stock. Stock's
  `Shift+number` (select the group and centre the camera on it) moves to `Alt+number`.
- `MaxSelection=` (default 40, 17–120; 16 or less leaves it stock): selections, and so
  control groups, hold more than stock's 16. The selection panel shows the first 16. (691256a)

Installed, not yet seen in game.

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
