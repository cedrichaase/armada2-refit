# Operating notes: the UI's style gallery and the stock UI tour (ui-style)

What an agent judging the UI's look needs, learned 2026-10-16 at 1280x720 and
1920x1080.

- **UI.asi into a running session**: `session start` launches the game at once, so
  install with `drive quit`, then `ui/bench-asi.sh <state.json> Demo.Style=1`, then
  `drive launch -nointro`. The ASI loader only reads plugins at launch.
- **The gallery** is behind the Credits (`Click the credits` on the main menu, about
  three seconds to draw). Esc leaves it. `drive move X Y` then a shot shows a live hover
  (the first button of each card, the toggles and sliders are live; everything else is
  drawn in a fixed state).
- **Over a mission**: Instant Action as `_enter-instant-action.md`; the gallery covers
  the screen over the HUD and the 3D view, good for judging the panels' translucency.
  A shot 1.5 s after LAUNCH catches the loading screen (the load at 1280x720 takes
  about 3 s; it reads 99% by then).
- **The stock UI, for reference** (shots in your scratch, never the repo): the main
  menu; Options (`Click "Options"`, Esc back); Game Setup (the Instant Action cube);
  a match as Federation (`Click at design 246,95`, then `246,109`; Borg is `246,123`).
  The HUD shows little until something is selected: click the starbase at the screen's
  centre to get the info panel and the build buttons.
- **Back to the menu from a match**: Esc opens the in-mission OPTIONS; `Click "Abort
  Mission"`, then the warning's Yes, which OCR does not read: click it by position
  (506,428 at 1280x720). A score screen follows: `Click "Done"`. The Game Setup screen
  keeps the AI slot from the last game.
- **The race panel art** is `Textures/RGB/<race>UI<n>.tga` (`fedui0..5`, `klingUI*`,
  `romUI*`, `cardUI*`, `borgUI*`, `speciesUI0..6`; the stock copies are the
  `.a2neb-backup`s where the texture pipeline replaced them). A contact sheet with
  `magick montage` shows each race's shapes at a glance. The per-race tooltip colour
  is `tooltipBackgroundColor` in `misc/gui_<race>.cfg`.
- Shell lines that mix variables with `magick`, `sed` or loops are refused by the
  agent sandbox: put them in a script file and run that.
