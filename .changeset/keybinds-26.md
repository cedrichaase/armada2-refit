---
ui: minor
qol: minor
testbench: minor
games: [armada2]
status: Installed, not yet seen in game
---

### Added
- UI: the Refit screen's **Keys** tab (`menus/refit_keys.rs`; issue #26): every keyboard key of the game's `Input.map` listed by what it does, under the file's sections, with a search field, the clashes named on each row ("Same key as: ..."), and a rebind by clicking a row and pressing the new key (with Ctrl, Shift or Alt; Esc cancels, *Stock key* puts stock's back). The command bar's grid keys (QOL.asi's `[Keys]`, `QOL_BarKeyName`) are listed and rebound on the same tab, live; the bar's own keys are dimmed while the grid keys are on.
- UI: `keys.rs` keeps the choices in the overlay (`RefitPlayer.ini [UI.Keys] <action>=<keys>`, `[QOL.Keys] CellN`) and writes them into `Input.map` (the stock file kept as `Input.map.a2neb-backup` on the first change, restored when no choice is left) at start, before the engine reads it, and after each change; a change of `Input.map` applies at the next start. `input.rs` records key presses by position (`note_press`); `note_message` takes the lParam. `install.sh --remove` and the release's uninstall put the stock `Input.map` back; the `ui` layer `replaces` `Input.map` (`a2mod`). Log lines `keys: ...`.
- QOL: `[Keys] Cell1`..`Cell15` in `QOL.ini` (shipped Q W E R T / A S D F G / Z X C V B): which key presses which cell of the command bar's grid, by `Input.map` name; read at start and by `QOL_SettingsChanged` (live). A name the grid cannot use keeps the cell's default and is logged. A cell whose key is a `[BarKeys] Rebind` row's FROM (until it applies) or TO gets no key. Export `QOL_BarKeyName(cell)`. `QOL.log`: `bar: keys ...` at start, `bar: Cell3 E -> F1` on a change.
- Testbench: scenario `ui-refit-keys` (search, a rebind of `Input.map`'s Zoom camera in to F6 kept across a relaunch, a grid key rebound live, Reset tab).

### Changed
- QOL: the `--- QOL BarKeys=1` line reads `-> the grid's keys ([Keys]) press the Refit bar's cells, patched`; scenario `qol-bar-keys` expects it.
