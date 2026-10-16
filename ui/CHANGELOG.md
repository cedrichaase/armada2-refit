# Changelog — ui

`UI.asi`: the runtime the game's HUD, menus and loading screen will be redrawn with, in
our own code (`PLAN.md`). Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. Details are in [`README.md`](README.md).

## 0.10.1 — 2026-10-17

### Fixed
- The briefing's objectives no longer run under their LCARS switches: the text starts
  after the switch's real width plus a gap (`theme::check_w`), not after a square box.

Installed, not yet seen in game.

## 0.10.0 — 2026-10-17

### Added
- `[HUD] TeamTexts` (default 1): the mission scripts' trade status ("Sent to Team 2"
  and the resources sent, in *Lifelines*) and team message, printed stretched in the
  default font by `Team::RenderUserTeamData`, are drawn by us: the trade status in a
  panel under the top-right buttons, the message under the resource strip.
- `[HUD] Icons` (default 1): our own flat icons, baked at build time into a
  signed-distance atlas, on the button bar's commands and menus and the top-right
  buttons, in the theme's colours, keeping the game's metaphors (chair, I-beam, brain,
  ...); special weapons and build items keep the game's art. 0: the game's art.

### Changed
- The info panel draws a builder's queue only when something is queued, and no
  construction bar at 0% for an idle builder: an idle starbase's panel is about a third
  shorter. The systems row is a little lower (34 units).
- The minimap's filters are words (SHIPS, STATIONS) and four state dots each (all,
  allies, enemies, neutral, the one in force lit) in place of the stock buttons'
  pictures; a click on a dot picks that state, a click on the word goes to the next.

Installed, not yet seen in game.

## 0.9.1 — 2026-10-17

### Fixed
- The campaign selection's rows could show the campaigns' names in the wrong order
  after an aborted mission: each row now takes the stock label that names its campaign,
  never the labels' order (UI.log notes `menus: campaign labels out of order` when they
  come otherwise).

Installed, not yet seen in game.

## 0.9.0 — 2026-10-17

### Added
- A mission loaded while the menu scene is up (from the campaign selection) keeps the
  scene under the loading screen and departs from it: the view's ships go to warp and
  the camera follows them into the tunnel (`Render3D_MenuWarp`, needs render3d 0.10.0).
  UI.log: `menus: the load departs from the scene <view>`.
- Frames of ours inside a loading step, after each SOD and texture loads (hooks on the
  calls of `LoadDatabase` and `LoadPixelData`), so the screen no longer stands still
  through a long step.
- UI.log names each stretch of 100 ms or more with no loading frame presented, with
  its step and what loaded in it (`loading: still`).

Confirmed in game 2026-10-17.

## 0.8.0 — 2026-10-16

### Changed
- Over the menu scene the campaign selection is a column like the main menu's: each
  campaign its faction's emblem (the HUD's race icons from `allrace.tga`; for the
  tutorials Starfleet Academy's patch, drawn) and its name, then Load Game and Previous
  Menu. Pointing at a campaign moves render3d's camera to its scene (needs render3d
  0.9.0). UI.log notes `menus: campaign emblems`.

### Fixed
- After a mission was aborted the campaign selection drew without the menu scene: the
  mission's picture was handed on from Options to the admiral's log to it. The main
  menu and the campaign selection now drop it (`backdrop::begin`).

Confirmed in game 2026-10-16.

## 0.7.0 — 2026-10-16

### Added
- `[Menus] Scene` (default 1): the main menu and the campaign selection stand over
  render3d's 3D scene, a fleet in orbit over a planet (needs render3d 0.8.0; without
  it they draw as before). Over it the main menu draws none of the stock art and takes
  no click off its column; its entries are lit text with an accent bar. The campaign
  cards stand in a row along the foot. UI.log notes `menus: scene <name> on`/`off`.

Confirmed in game 2026-10-16.

## 0.6.0 — 2026-10-16

### Added
- `[HUD] Tooltips` (default 1): the tooltips the engine shows at the pointer (an
  object's name and description over the 3D view) drawn by us in the theme at the
  screen's resolution, in place of stock's, stretched on the 1600x1200 canvas.
- `[HUD] SelectionBubbles` (default 0): with `Selection=1`, the engine's 3D selection
  ellipsoids are switched off (`RenderSelectionEffects` returns at once; render3d
  follows), the brackets alone mark the selection.

### Changed
- The selection brackets are thinner (1.5 units, 2 under the pointer).
- The resources read dilithium, metal, latinum, biomatter, crew, officers.
- The unit view's frame is square in every theme (LCARS's rounded one cut into the
  picture's corners).
- The Admiral's Log no longer offers Save to text.

### Fixed
- No cursor over a mission's opening briefing, so its O.K. could not be aimed at: the
  box of ours stays the mouse's owner and its stock `Render`, which names the cursor,
  was skipped. The owner's cursor (else the arrow) is named for it, and the arrow
  shows whenever a box of ours is up and the engine names none.
- The cursor was drawn under the overlay layer (the boxes, dropdowns, tooltips).
- Under the LCARS frame the button bar, the info panel (one unit and a group), the
  hover card and the minimap's filter buttons ran into the frame's elbows and bars:
  each is laid out inside the frame (`theme::frame_margins`).

Installed, not yet seen in game.

## 0.5.0 — 2026-10-16

### Added
- The info panel's close-up, `[HUD] InfoCamera=1`: a single craft drawn live by the
  engine's renderer from a camera of the UI's, in place of its wireframe (needs
  render3d 0.7.0; without it the wireframe stays).
- `[HUD] InfoScale` (1.3): the info panel's size.

Installed, not yet seen in game.

## 0.4.0 — 2026-10-16

### Added
- Multiplayer HUD panels: `[HUD] Comm` (the comm panel), `DropPlayer`, `LoadWait`; the
  Internet lobby screen; the lobby's game rows and player list in the control model;
  `bench-asi.sh --online`, `--for` (eed02e3).
- The unit view placed by the UI beside its info panel, `[HUD] UnitViewSize`; the
  engine's cursor kept square at any aspect when `[UI] Cursor=0` (8c34190).
- `ui/supersede.sh`, run by `ui/install.sh`: takes `HUD.asi`, `GridLayout.asi` and
  `Menus.asi` out and puts the HUD's file-based fixes back to stock;
  `ui/bench-legacy.sh` puts an old plugin back in a bench clone (8c34190).
- `[UI] Defaults`: a reinstall moves keys still at the old defaults to the new ones,
  and keeps every value the player changed (`ini-merge.py`) (8c34190).

### Changed
- **Every part of the UI is on by default**: every `[HUD]` panel, `[Menus] Enable`,
  `[Loading] Enable`, `[UI] Cursor`. A key at 0 gives that part back to the engine
  (8c34190).
- HUD panels draw only in their view's render list (no HUD over a cutscene); the
  objectives box keeps the mouse while it shows, so Return closes it as stock; the main
  menu's sides are the widescreen plate when one is installed (8c34190).

### Fixed
- Typing into a field of ours over a hosted dialog was doubled or lost; Enter in a
  field sends (Game Setup's chat); Game Setup's multiplayer rows (eed02e3).
- Graphics Settings drew its display-mode and driver fields twice (2e2298e).
- The loading screen labelled Instant Action "Multiplayer" (14ec9a0).

Installed, not yet seen in game.

## 0.3.0 — 2026-10-16

### Added
- `ui/install.sh`, run by `./install`; `a2mod` switches `UI.asi`/`UI.ini` as the `ui`
  layer. A reinstall keeps the player's `UI.ini` values (`ini-merge.py`) (04194ee).
- The HUD's race: `[UI] Theme=auto` draws the player's race's theme in a mission
  (`engine/race.rs`) (9bf63fe).
- HUD panels of ours, each off by default: `[HUD] Info` (the info panel: one unit, a
  builder and its queue, a group), `Minimap` and `MinimapSize`, `Selection` and
  `SelectionGauges` (the selection overlay), `UnitView` (a frame around the unit view),
  `Resources`, `Buttons` (the top-right buttons), `Messages` (the message and
  objectives boxes), `Pause`, `Replay`, `Overlay` (the centre messages) and `Chat` (the
  chat lines and the text entry) (9bf63fe, 9acbb51, a088117).
- Menu screens of ours (`[Menus]`): Single Player, Mission Select, Load Game, Save Game,
  the Admiral's Log, the Technology Tree; Options (from the main menu and in a mission),
  Game, Graphics and Sound Settings, the message and yes-or-no boxes, net waiting, help,
  the licence; Game Setup, Advanced Options, Map Select, Multiplayer Connection, Manual
  IP and the other small multiplayer dialogs. The shell's scroll bars and combo boxes
  are read into the control model (4573f6c, e9cc6b4, 4ca533d).
- A loading screen of ours, `[Loading] Enable` (default 0): the map's sky, the
  mission's title, briefing and objectives, the players and their multiplayer load,
  frames between the engine's 5% steps; `[Loading] Hold` for the bench (2e9bb2b).
- `bench-asi.sh --scene`, `--dplay`.

### Changed
- The button bar is drawn through the theme and placed against the UI's info panel
  when that is on (9bf63fe).
- The generic `widgets` menu screen uses the shared widgets; `menus/local.rs` is gone
  (e9cc6b4).

### Fixed
- A dialog opened on a button down kept the mouse captured, and the next dialog took no
  click (4573f6c, 4ca533d).
- A screen under the top dialog took the pointer: "Save Successful!"'s O.K. could not be
  clicked over the save list (defa59f).

Installed, not yet seen in game.

## 0.2.0 — 2026-10-16

### Added
- Widgets: text field (with the clipboard), scroll area, list, dropdown, tabs, check
  box and radio, image button, number spinner, tooltip, modal; keyboard focus (Tab,
  Shift+Tab, Enter, Esc) and an overlay layer (481b796).
- Our own cursor, from the game's cursor art, square at any aspect: `[UI] Cursor`
  (default 0), `CursorSize` (481b796).
- The style (`STYLE.md`): one theme per race and one for the shell, drawn from tokens
  (`widgets/theme.rs`); shader kind 4 (chamfered or rounded corners, stroke, glow,
  elbow, stripes); `[UI] Theme` (default `auto`) (08e2901).
- `[Demo] Shell=1` shows a widget gallery; `[Demo] Style=1` a style gallery (481b796,
  08e2901).
- The HUD, a panel at a time (`HUD.md`): one switch skips a stock panel's drawing and
  mouse while it keeps simulating; engine reads for the button bar, sprites, sounds,
  the selection and resources; our own button bar as a 5x3 grid with its keys, which
  stands down while `GridLayout.asi` is loaded. `[HUD] Enable`, `Bar` (default 0),
  `BarKeys`, `BarLabels`; `bench-asi.sh --no-grid` (0e0cac9).
- The menus (`MENUS.md`): the stock shell dialogs hosted hidden and drawn by the UI
  (the dialog host, a registry of the shell's widgets, the control model, a generic
  mirrored screen and a spec for the main menu); the shell's display mode raised to
  the desktop size; the in-mission menus over the last frame, dimmed. `[Menus] Enable`
  (default 0), `Generic`, `Specs`, `EscapeReturns`, `Trace`, `Host`;
  `bench-asi.sh --menus` (f0fe918).

### Fixed
- A click no longer gives the keyboard focus to a button, toggle, slider, check box or
  tab: it kept every key from the engine and Esc from the screen (345bc74).

Installed, not yet seen in game.

## 0.1.0 — 2026-10-16

### Added
- `UI.asi`, a `no_std` Rust plugin built by `ui/build.sh`, with `UI.ini` (`[UI] Enable`,
  `Log`, `Scale`; `[Demo] Mission`, `Shell`, `Loading`). Not in `./install` or `a2mod`
  yet; `ui/bench-asi.sh` puts it into a bench clone.
- The 2D renderer on the Direct3D 9 device behind d3d8to9: one shader pair
  (`shaders/ui.hlsl`) for solid, gradient, image, nine-slice, signed-distance text and
  rounded rectangles, batched, scissor-clipped, the device's state saved and restored.
- Text: Noto Sans Condensed Medium and SemiBold (OFL-1.1, vendored in
  `platform/vendor/noto-sans-condensed-2.015/`), baked at build time by `ui/fontbake`
  into signed-distance atlases, with GPOS kerning and word wrap.
- Immediate-mode widgets sized in units of a 1080-pixel-tall screen: panel, window,
  label, note, button, toggle, slider, progress; `Anchor` and `Stack` layout.
- Input from the game window's messages, with capture: what the UI takes (clicks and
  the wheel over its panels, keys for a focused field) never reaches the engine.
  The engine's cursor can be hidden a frame at a time, and its shape read.
- Three drawing contexts: a mission's frame after the HUD and chat; a screen loop of
  our own in the front end (`[Demo] Shell=1` puts one in the Credits' place); the
  loading screen, with the load's progress.

Installed, not yet seen in game.
