# Changelog — menus

`MenuScale.asi`: the 800x600 shell menus scaled to fill the screen, embedded in the game
window, with hi-res outpainted backdrops. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The full write-up is in
[`README.md`](README.md).

## 2.1.0 — 2026-09-25

### Added
- `EscapeReturns=1` (default): Esc in the in-mission menu acts as Return to Game, so
  Esc both opens and closes it. Needs `Embed=1`; `0` leaves Esc stock. (`460a06a`)

Confirmed in game 2026-09-25.

## 2.0.2 — 2026-09-25

### Fixed
- Esc reopens the in-mission Options menu after it has been closed. An embedded menu
  left the keyboard focus `NULL` when it returned, so every key was dropped until a
  restart. The focus is now restored to the window that held it. (`54ad62c`)

Confirmed in game 2026-09-25.

## 2.0.1 — 2026-09-25

### Fixed
- The Admiral's Log's tab panes (`CreateDialogParamA`) are embedded as children of the
  log, so they close with it. They had outlived it, owned by the 3D window, and the
  score table stayed on top of every menu afterwards.
- The log's owner-drawn buttons (player list, tabs, Save/Done, and those in the Ships
  and Battles panes) are scaled and placed with the rest of the screen, instead of
  drawing 1:1 in the top-left corner. New `CreateWindowExA` hook, and a design-sized
  `WM_DRAWITEM` surface.
- A dialog inside a letterboxed dialog is fitted through that dialog's placement
  rather than the screen's. (`2f0f9b9`)

Confirmed in game 2026-09-25.

## 2.0.0 — 2026-09-25

The plugin is renamed from `MenuScale` to `Menus`, because scaling is about a third of
what it does. Its code is unchanged apart from names and one guard: before the rename
it disassembled identically to what was installed.

### Changed
- `menuscale.c` → `menus.c`, and the installed files `MenuScale.asi`/`.ini`/`.log` →
  `Menus.asi`/`.ini`/`.log`. The ini section is now `[Menus]`, the plates folder
  `Menus/`, and backdrop dumps are `Menus-backdrop<n>.bmp`. All ini keys are unchanged.
- `a2mod` layer `menu scale` → `menus`.

### Added
- `Menus.asi` stands down, and logs why, if a `MenuScale.asi` is beside it: the ASI
  loader loads every `*.asi`, so both would hook everything.
- `install.sh` removes the old `MenuScale.*` files on install and on `--remove`. When a
  plate isn't built in the current checkout, it keeps the installed one instead of
  claiming the sides will be black.

MAJOR because the installed file names and the ini section changed. Migrated in the
game directory on 2026-09-25: `MenuScale/` renamed to `Menus/` (plates byte-identical),
`install.sh` run, and the `a2mod` manifest entries renamed. The plugin was load-tested
under Proton's Wine (new names read, legacy guard fires). Confirmed in game 2026-09-25.

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
- `Underlay=0` (`ab6ceab`) and `NoiseFloor=` (`96875e5`). Both confirmed in game.
- Main-menu plate redone to be free of stars: seed 6 with `fade=`/`dehaze=`. Accepted
  by the user (`5ba10e4`, `3debaed`).
- Campaign screen: the ghost bar beside the Tutorials panel fixed, and `keep=`
  rectangles added. `install.sh` renames files into place (`ed012a1`, `101daba`).
- Moved from `tools/menuscale/` to `menus/` (`c46a493`).

### 2026-09-24
- Hi-res outpainted backdrops (`e753f42`), the campaign screen from its open field
  (`a51638a`), and `N.soften=` for the Tutorials glow (`f5dd368`).

### 2026-09-23
- `Embed=1`: one OS window for every menu. Fixed the blank screens caused by leaked
  DCs, and kept the OS cursor over embedded menus. Confirmed in game (`0c42d5a`,
  `e3939d1`, `475aa3b`).

### 2026-09-21
- `MenuScale.asi`: display mode raised, shell scaled. Confirmed in game (`b937b2c`).
- Fixed: hover and clicks landed on stock positions, because modal dialogs bypass the
  message loop. Input mapping moved into the window procedure (`e67902c`).
- `stop-game.sh`: Wine names the process `Main`, so `pkill -x Armada2.exe` had been
  killing nothing (`57de920`).
