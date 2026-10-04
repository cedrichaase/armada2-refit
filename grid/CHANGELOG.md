# Changelog — grid

`GridLayout.asi`: the button bar as a fixed 5×3 grid with one key per cell, by keyboard
position. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions".
Newest first. How and why: [`README.md`](README.md).

## 1.0.0 — 2026-10-04

### Added
- `GridLayout.asi`, `GridLayout.ini`: the bar is a 5×3 grid in every palette mode, keys
  QWERT / ASDFG / ZXCVB by scancode; T is cancel and G back in every menu, B the 13th
  build item. The top level puts the build/research/evolve menu first in the top row,
  commands in the middle row and special weapons in the bottom row; submenus use the
  commands' own `preferredPosition`; build lists fill the left four columns column by
  column.
- The bar's stock keys (build F-keys, special weapons, command letters, the menu toggles)
  are off while the grid is on. A grid key does what a click does: nothing on a
  disabled button, else the click sound and the press. Nothing fires while typing
  (chat), with Ctrl or Alt held, or without focus.
- The key in each button's corner, `Labels=` and `LabelSize=` (percent, default 87).
- `Place=beside` (default): at 16:9 and wider the grid sits between the minimap and the
  info panel, which moves right towards the unit view; where there is no room it stays
  above the info panel. `Place=above` keeps it there always.
- `[Cells] name=key` moves single buttons. `Enabled=0` leaves the bar stock.
- `install.sh` (`--remove`); `./install` runs it, `a2mod` switches it as layer `grid`.

Installed, not yet seen in game.
