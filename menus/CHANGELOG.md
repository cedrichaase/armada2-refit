# Changelog — menus

`MenuScale.asi`: the 800x600 shell menus scaled to fill the screen, embedded in the game
window, with hi-res outpainted backdrops. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The full write-up is in
[`README.md`](README.md).

## 1.0.0 — 2026-09-25

Baseline: the first versioned release. Everything below is confirmed in game.

- The front end's hard-coded 800x600 display mode is raised to the desktop size, in
  memory only. The shell is drawn fitted and centred, and mouse input is mapped back.
- `Embed=1`: every menu, the in-game Options included, is created as a child of the
  game window, so the game is one OS window from launch to exit.
- `Backdrops=1`: outpainted plates for the main menu (seed 6, `fade=12`,
  `dehaze=100`) and the campaign selection screen (`field=`, `clone=`, `keep=`,
  `2.soften=`).
- `NoiseFloor=6`: the hover Binks no longer dim their rectangle.
- `Underlay=0`: switching menus no longer flashes the next background 1:1 in the corner.
- Open: the Admiral's Log (modeless, not hooked) and unscaled text boxes.

## Before versioning

### 2026-09-25
- `Underlay=0` (`59f27dc`) and `NoiseFloor=` (`d16fb2d`). Both confirmed in game.
- Main-menu plate redone to be free of stars: seed 6 with `fade=`/`dehaze=`. Accepted
  by the user (`79695b2`, `8a3562c`).
- Campaign screen: the ghost bar beside the Tutorials panel fixed, and `keep=`
  rectangles added. `install.sh` renames files into place (`e8f7ab9`, `d3e1ccb`).
- Moved from `tools/menuscale/` to `menus/` (`c3ae017`).

### 2026-09-24
- Hi-res outpainted backdrops (`3c8d736`), the campaign screen from its open field
  (`15d8cf0`), and `N.soften=` for the Tutorials glow (`e9ecc15`).

### 2026-09-23
- `Embed=1`: one OS window for every menu. Fixed the blank screens caused by leaked
  DCs, and kept the OS cursor over embedded menus. Confirmed in game (`a2d14a8`,
  `fda8bcd`, `e70a7de`).

### 2026-09-21
- `MenuScale.asi`: display mode raised, shell scaled. Confirmed in game (`4caeeed`).
- Fixed: hover and clicks landed on stock positions, because modal dialogs bypass the
  message loop. Input mapping moved into the window procedure (`f5eb1f1`).
- `stop-game.sh`: Wine names the process `Main`, so `pkill -x Armada2.exe` had been
  killing nothing (`4617a77`).
