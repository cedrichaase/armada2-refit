# Operating the menus when UI.asi hosts them (`ui/bench-asi.sh --menus`)

What an agent driving the front end with the UI's menus on needs to know (`ui/MENUS.md`).

## Setting up

- `Setup: ui/bench-asi.sh --menus` in a scenario, or by hand on a session started with
  `--no-launch`: `ui/bench-asi.sh ~/.cache/a2test/sessions/<ID>/session.json --menus`
  (the state file is `session.json`), then `a2test drive --session ID launch -nointro`.
  It sets `[Menus] Enable=1` and takes `Menus.asi` out of the clone: with Menus.asi
  loaded the UI's menus stand down.
- Extra keys go after it: `Menus.Trace=1` (UI.log gets each dialog's messages, the
  actions the UI sends, and every 5 s the hidden dialogs' surfaces as
  `UI-dlg<depth>-surface.bmp` / `-frame.bmp` in the game directory: what the game drew,
  before the UI composes it), `Menus.Host=0` (the dialogs run stock, only traced: the
  baseline to compare a trace with), `Menus.Generic=widgets`, `Menus.Specs=0`.
- To relaunch the game in the same session after rebuilding, end only your game: find
  the `Armada2.exe` process whose environment names your session directory, and kill
  that one. **Never `pkill -f Armada2.exe`**: it matches other jobs' games (and its
  own shell).

## Where things are

- The design space (800x600) is fitted to the screen's height, centred, as Menus.asi
  fits it, and UI.log logs `menus: MoveWindow 800x600 @0,0 -> 960x720 @159,0` at
  1280x720. The bench reads that line, so `Click at design X,Y` and the `ui.json`
  targets land on every mirrored screen, and on the main menu's faded art too.
- **The main menu is a spec**: a column of buttons on the left (at 1280x720: SINGLE
  PLAYER 228,258; INSTANT ACTION 228,302; MULTIPLAYER 228,346; OPTIONS 228,390;
  REPLAY INTRO 228,434; CREDITS 228,478; EXIT 228,522). OCR also reads the words
  painted into the faded art behind it ("OPTIONS" at 764,484), and `Click "OPTIONS"`
  clicked the art; the art works too (it passes clicks to the stock screen), but the
  column is what a scenario should click, by position.
- **Over the menu scene** (`[Menus] Scene=1` with render3d 0.8.0 or later; UI.log `menus: scene
  main on`) the main menu has no art behind it and takes no click off the column: a
  design point clicks nothing. `Click the single player emblem` and the other
  `ui.json` targets with a `label` click the UI's own entry by OCR then; the column's
  positions above are unchanged. The campaign selection is a column at the left too,
  at 1280x720 its rows at y 270 (Tutorials), 322, 374, 426 (Borg), x 220: `Move the
  mouse to` a row moves the camera to that campaign's scene (Render3D.log `menu: view
  <name>`; wait ~4 s for the 2.5 s move, ~8 s for the Klingons to decloak).
  `Menu\map.ini` in the clone retunes the map live (`render3d/README.md`); the
  projector math there is plain look-at: a camera on +z looking down -z has screen
  right along -x.
- Game Setup's player rows: `Click at design 124,115` opens player 2's list,
  `124,143` picks "Computer AI:Easy"; `246,95` / `246,123` set player 1's side to Borg
  (as in `ui-runtime.md`).
- Graphics Settings' Bump Mapping switch: design 570,240. Sound Settings' Unit Speech:
  design 400,403. Save Game's name field: design 600,516 (it opens holding the map's
  name; End and Backspace clear it); its Save button design 400,579, Load Game's the same.

- **Credits** (the main menu's column, 228,478 at 1280x720) are a screen of ours over
  the menu scene: `key space` turns the page, `key escape` returns to the main menu
  (UI.log `credits: left (0) at page N`).
- **The Admiral's Log's tabs** at 1280x720: Score 132, Military 277, Economy 422,
  Timeline 567, Battles 712, Ships 857, Build 1002, Tally 1147, all at y 85. The
  Ships page's arrows are at the page's right corners (1201,122 and 1201,630).
- **Game Settings' "Command Bar"** writes `UIPlayer.ini` in the clone's game
  directory; the next mission hides the bar (UI.log `bar: hidden as the player set
  it`).
- **Stopping only your own game** to relaunch after a rebuild: the processes whose
  `/proc/<pid>/environ` names your session directory (a script, as the sandbox refuses
  the loop inline).

## Traps

- A screen that looks frozen is the first thing to suspect the host of: dump the
  surfaces with `Menus.Trace=1` and compare `UI-dlgN-surface.bmp` (what the game drew)
  with the screenshot (what the UI shows). Game Setup showed its first picture for
  ever until the host stopped reading surfaces through their DCs (`ui/MENUS.md`).
- `Press Escape` in the in-mission menu returns to the game (`EscapeReturns`); in any
  other dialog it is `IDCANCEL`, which most screens ignore, as in stock.
- The bench's `Type` sends lower case.
