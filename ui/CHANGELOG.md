# Changelog — ui

`UI.asi`: the runtime the game's HUD, menus and loading screen will be redrawn with, in
our own code (`PLAN.md`). Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. Details are in [`README.md`](README.md).

## 0.24.0 — 2026-10-29

### Added
- A frame of the UI's `gpu` calls can be recorded for a replay with no game
  (`gpu/README.md`, "Record and replay"): asked for with `Gpu.record` beside the exe, on
  Display8's Vulkan backend only, written to `Dumps\<name>\UI.asi.a2s`. The recorded
  frame ends at the engine's `Present`, where the UI dumps it as presented
  (`Dumps\<name>-game-*-present.ppm`) for the replay to be compared with. `239f29c`

Installed, not yet seen in game.

## 0.23.0 — 2026-10-25

### Fixed
- The game crashed before the first movie with Display8's stub and no cutscene proxy
  (the release package): `Program::PlayIntroMovie`'s 640x480 lookups are raised to the
  desktop's size with `[UI] Native=1` (`engine/intro.rs`), as the proxy does; with the
  proxy installed they stay its. `6d7180d`

### Added
- With `[UI] Movies=1` and no cutscene proxy, the launch reels and the intro are drawn
  by the UI, fitted between black bars: `NextBinkFrame`'s `BinkCopyToBuffer` call is
  re-pointed and the frame handed to `ui_present_movie`. `6d7180d`

Installed, not yet seen in game.

## 0.22.0 — 2026-10-23

### Added
- `[UI] KeepClip=1` (on as shipped): in a mission under Wine, UI.asi hands Wine's clip
  rectangle back to `ClipCursor` at most every 500 ms while the pointer moves and the
  game is the foreground window, so Wine clips the pointer to the full-screen game
  again after a screenshot's overlay let it go (`input::keep_clip`). For the pointer
  that escaped into Omarchy's bar and left the game's cursor standing (README.md,
  "Focus and the pointer"). `PointerLog=1` logs the first re-clip.

Confirmed in game 2026-10-23.

## 0.21.0 — 2026-10-23

### Added
- `[UI] PointerLog=1` (off as shipped): `UI.log` says each change of the game window's
  foreground, active and focus state, with the pointer, the engine's mouse buttons and
  whether its pan holds the pointer, and each jump of the pointer between two frames
  that the pan did not make ("focus: ...", "pointer: jumped ..."). For the cursor that
  jumps after the game lost the focus (README.md, "Focus and the pointer").

Installed, not yet seen in game.

## 0.20.1 — 2026-10-23

### Fixed
- `build.sh` writes `src/shaders/mod.rs` and `build/UI.asi` through temp files of its
  own, renamed into place: two builds of one tree at once (`a2test -j` installing one
  checkout twice) failed on the shared `mod.rs.new`.

Installed, not yet seen in game.

## 0.20.0 — 2026-10-23

### Added
- The placement markers in the theme (`hud::placement`, `[HUD] Placement=1`): a
  station's footprint green with corner brackets where the place is clear, red and
  hatched where it is not; the cells other objects take as amber tiles fading with
  distance from it, and those under it red and crossed. `Placement=0` draws the
  engine's sprites. Validity stays the engine's. `UI.log`: "placement: footprint
  blocked, ...". `gfx::Draw::poly`, `line_soft` and `band` draw them.

### Fixed
- The footprint under a station being placed was not drawn when the map had more than
  256 taken cells (`RenderFootprints` draws one quad per taken cell before the
  footprint, and the UI kept 256 quads a frame): the cells are now recorded apart, up to
  8192, with their camera, and culled to the viewport when drawn.

Confirmed in game 2026-10-23.

## 0.19.2 — 2026-10-23

### Changed
- `ini-merge.py`'s defaults upgrade reads `Defaults=` from whichever section of the
  shipped file has it, with the upgrades kept per file (`[UI]` for `UI.ini`,
  `[Display8]` for `Display8.ini`); `UI.ini`'s merge is as before.

Installed, not yet seen in game.

## 0.19.1 — 2026-10-23

### Fixed
- On Vulkan `gpu`'s read-back (the frame dumps) reads host-cached memory, not the card's
  memory through the PCIe window (`gpu/README.md`, "Read-backs").

Confirmed in game 2026-10-23.

## 0.19.0 — 2026-10-23

### Added
- The UI on `gpu`'s Direct3D 12 backend when Display8 draws on Direct3D 12: its
  pipelines carry their DXIL, and what took Display8's Vulkan device takes either of
  Display8's own (`gpu::Gpu::on_display`); the log names the backend.

Installed, not yet seen in game.

## 0.18.0 — 2026-10-23

### Added
- The UI on `gpu`'s Vulkan backend when Display8 draws on Vulkan (`gfx::attach` takes
  either device): the back buffer is Display8's image, the engine's textures (minimap,
  radar sprites, order ring and footprints, the unit view's video) its mirrors, a dump
  reads the frame before the gamma pass; positions leave out Direct3D 9's half pixel
  there.

Confirmed in game 2026-10-23.

## 0.17.2 — 2026-10-23

### Fixed
- A placement's build footprint is drawn again with Display8 in stub mode: the three
  `ST3D_Sprite::DrawQuad` calls in `PlaceholderFeature::RenderFootprints` are taken
  under `[HUD] Overview` as the order ring's are, and drawn by the UI with each
  sprite's rectangle of its texture (`MBuild`'s quarters) and a texture slot per engine
  texture (`engine/leftovers.rs`, `hud/leftovers.rs`; HUD.md, "The engine's last
  draws"). `UI.log`: "leftovers: build footprint at ...".

Confirmed in game 2026-10-23.

## 0.17.1 — 2026-10-23

### Added
- The UI shader's DXIL for Direct3D 12 (`src/shaders/ui_dxil.rs`), from
  `render3d/hlsl-rs.sh`; the d3dbc and SPIR-V are byte for byte as before.

Installed, not yet seen in game.

## 0.17.0 — 2026-10-23

### Added
- The engine's last draws in a mission are the UI's (`engine/leftovers.rs`,
  `hud/leftovers.rs`; HUD.md, "The engine's last draws"), each in the engine's look:
  the selection drag box, the height posts, the order ring (`mRenderGridFX`,
  `mRenderWhere`, drawn through a new additive pipeline, `Draw::quad_additive`),
  `ScreenFade`'s fade, and the unit view's Bink video from the engine's texture. New
  `[HUD]` keys `Overview`, `Fade`, `UnitVideo` (default 1).
- Movies through the UI: `UI.asi` exports `ui_present_movie(bgrx, w, h, pitch)`, which
  the cutscene proxy hands each full-screen frame to; the UI draws it fitted at the
  next `Present` (`src/movie.rs`; README.md, "Movies"). `[UI] Movies` (default 1).

### Changed
- The in-mission menus draw over a copy of the last mission frame the UI keeps (a GPU
  copy at the end of each mission frame); the front buffer is read back only when
  there is none (MENUS.md, "The in-mission menus").

### Fixed
- Choosing a display mode in Graphics Settings in the middle of a mission no longer
  kills the game with a stack overflow: the arrow over hosted dialogs and screens of
  ours comes from a patch of the engine's own `WM_SETCURSOR` answer instead of a
  subclass of the 3D window, which looped with DXVK's window hook once the device was
  re-created (README.md, "The front end: screens of our own").

Confirmed in game 2026-10-23.

## 0.16.0 — 2026-10-23

### Changed
- `shaders/ui.hlsl` is modern HLSL, built for Direct3D 9 as before and for SPIR-V
  (`src/shaders/ui_spv.rs`) (729f9bb).
- The 2D renderer, the close-up's pictures, the menus' backdrop, the minimap's engine
  textures and `UI.dump` go through `gpu/`: the frame's state is saved with the
  recorded state block instead of `D3DSBT_ALL`, vertices go through gpu's ring
  buffers, and textures are made again by gpu after a `Reset` or a new device (caef061).
- Its pipelines turn the clip planes off; binding the same pipeline again sets nothing; a
  colour clear writes every channel (gpu, 1b63c7f).

### Removed
- `src/d3d` (now `gpu::d3d9::raw`).

Measured on the bench: the showcase's presented frame with the HUD is the same pixels
as before (RMSE 0). Confirmed in game 2026-10-23.

## 0.15.0 — 2026-10-22

### Added
- Frame dumps on request: a `UI.dump` file beside the exe makes the next `Present`
  write the back buffer, the frame as presented, as PPM into `Dumps\` with a sidecar
  `<name>-<frame>-ui.txt`. The d3d8 device's `Present` is chained beside `Reset`
  (`src/dump.rs`, `engine/device.rs`; README.md, "Frame dumps").

Confirmed in game 2026-10-22.

## 0.14.0 — 2026-10-22

### Added
- `[Bench] Clock=` in `UI.ini` (default `wall`, as before): `bench` makes `Ui::time()`
  the simulation's time and settles every transition at once (hover fades, a panel
  opening, tooltip delays through `Ui::waited`, the unit view's camera), for
  pixel-identical frames on the test bench. Lists re-read on a timer (resources, the
  loading screen's players) now time it on the wall clock (ecbb58b).

Confirmed in game 2026-10-22.

## 0.13.4 — 2026-10-21

### Changed
- `UI.asi` carries a version resource (`VERSIONINFO`): product, description, the
  layer's version as the file version and the project's as the product version, which
  Windows shows in the file's Properties. Built by `platform/versioninfo/build-res.sh` (915cad8).

Confirmed in game 2026-10-22.

## 0.13.3 — 2026-10-18

### Fixed
- Game Settings' "Command Bar" at Shown no longer starts a mission with the bar
  hidden. The stock toggle (`NextPaletteMode`) switches the palette mode (`+0x110`)
  between the bar (1) and the floating palette (0), but the setting was checked against
  the component's hidden flag (`+0x16`), which the engine also sets for other reasons
  (under the opening cinematic, before the first selection): with the bar already up the
  toggle was pressed and took it away. It is now checked against the mode, once the bar
  is in the view; UI.log says "bar: shown as the player set it (palette mode M, hidden
  flag F)", with ", toggled" when it pressed.
  `testbench/scenarios/ui-hud-bar-after-cinematic.md` checks it.

Confirmed in game 2026-10-19.

## 0.13.2 — 2026-10-17

### Fixed
- Our cursor obeys a hidden engine cursor: `RefreshDisplay`'s draw of the cursor
  (0x6246fa) is wrapped when the UI loads (`engine::cursor::install_probe`), and while
  something before it skips that draw (Scene.asi's `Cursor=0` / `cursor off`) the UI
  draws none either (`cursor: Mission draws none`). The engine's own is now held back
  at that call rather than by emptying `+0x60`.
- No cursor on the loading screen: the engine draws none there, and ours is drawn only
  where the engine would draw one.

Installed, not yet seen in game.
## 0.13.1 — 2026-10-18

### Changed
- The log names any default-pool object of the UI's still held at a device `Reset`
  (`d3d::live_default`).

Confirmed in game 2026-10-18.

## 0.13.0 — 2026-10-18

### Added
- `[UI] Native=1`: a game whose display mode is still a stock one (800x600 with no
  `ARMADA.PRF`, 1024x768 as GOG ships it) runs at the desktop's, so the menus, the
  loading screen and the missions share one mode from the first launch; a mode chosen
  in Graphics Settings is kept (`engine/display.rs`, on the call of
  `InitializeGraphicsSystem`).

Confirmed in game 2026-10-18.

## 0.12.1 — 2026-10-18

### Fixed
- On Windows, without DXVK, the UI drew nothing: Direct3D 9 refused `PS_UI` as vkd3d
  writes it ("gfx: could not make the shaders or the state block"), and the menus ran
  on a black screen. The shaders are built with `hlsl-rs.sh --fix`; both are made on
  Windows 10's runtime (render3d/README.md, "Shaders on Windows").
- On Windows, starting a mission ended in "Fatal error in Direct3D. Please reboot.":
  the UI's objects kept the shell's device alive. `engine/device.rs` chains
  `DestroyRenderSurfaces` and lets go of everything first, and chains `Reset` from the
  first frame, letting go of the state block, which a native `Reset` refuses to keep.
  The log names by kind any object of ours still held then (`d3d::live`).

Confirmed in game 2026-10-18.

## 0.12.0 — 2026-10-17

### Added
- `[HUD] LayoutLog` (default 0), for the test bench: every panel's rectangle, as
  `Ui::panel` draws it, written to UI.log when the HUD's layout changes and has held
  for five frames ("layout: WxH | info x,y,w,h | bar ..."; `hud/layout.rs`, HUD.md,
  "The layout log"). Nothing is kept or written at 0.

Installed, not yet seen in game.

## 0.11.1 — 2026-10-17

### Changed
- The bottom row (`HUD.md`, "The bottom row"): at 16:9 and wider the info panel moves
  right just far enough for the bar to sit at the bottom beside the minimap, at a
  fixed place; where the row has no room for both (4:3) the bar sits above the minimap
  and the panel is centred clear of the minimap and the unit view. Before, the bar sat
  above the info panel at 16:9.
- A group's tiles are smaller (46 x 40 units, were 70 x 78) with a plain hull strip,
  in rows as wide as the one-object panel, three rows before they grow wider and no
  wider than the room beside the bar: 30 units take three rows of ten, not four of
  eight.

Installed, not yet seen in game.

## 0.11.0 — 2026-10-17

### Added
- `[Menus] Credits` (default 1): the Credits are ours: `Credits.txt` page by page over
  render3d's menu map, the camera touring its views; the four logos square in the
  corners. Esc returns to the main menu, Space, Enter, the right arrow or a click turn
  the page, the left arrow goes back (`credits.rs`).
- Game Settings has a row of ours, "Command Bar" (Shown or Hidden): the button bar's
  toggle, kept between sessions in `UIPlayer.ini` (`[HUD] BarHidden`, a file of the
  player's that no install writes) and applied at the start of each mission
  (`prefs.rs`).
- Over the minimap the cursor is the warp cursor (`c_warp`) while a selected ship of
  the player's can warp, the move cursor (`c_move`) while none can.
- The main menu's Single Player, Instant Action and Multiplayer and the campaign
  selection's Tutorials and Federation rows, Load Game and Previous Menu play the
  shell's hover sound, as the stock buttons do (`sound::shell_hover`).

### Changed
- Every full-screen front-end screen (Options, Instant Action's Game Setup,
  Multiplayer and its lobby, Load Game, the Admiral's Log from the front end) stands
  over the menu scene, dimmed, in place of the old main menu's art.
- The Admiral's Log's Military, Economy, Timeline, Battles, Ships, Build and Tally
  pages are set from their labels in the theme's type, rows striped, without the frame
  art; Timeline's graph and the Ships portraits are cropped from the stock page, and
  our arrows and the wheel scroll the stock lists.
- The Internet lobby and Game Setup (Instant Action's and multiplayer's) are laid out
  in a box no wider than 16:9, centred.

### Removed
- The top-right help and palette buttons, and with them "show more": the row is menu,
  comm, idle unit, view and objectives.

Confirmed in game 2026-10-17.

## 0.10.1 — 2026-10-17

### Fixed
- The briefing's objectives no longer run under their LCARS switches: the text starts
  after the switch's real width plus a gap (`theme::check_w`), not after a square box.

Confirmed in game 2026-10-17.

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

Confirmed in game 2026-10-17.

## 0.9.1 — 2026-10-17

### Fixed
- The campaign selection's rows could show the campaigns' names in the wrong order
  after an aborted mission: each row now takes the stock label that names its campaign,
  never the labels' order (UI.log notes `menus: campaign labels out of order` when they
  come otherwise).

Confirmed in game 2026-10-17.

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

Confirmed in game 2026-10-17.

## 0.5.0 — 2026-10-16

### Added
- The info panel's close-up, `[HUD] InfoCamera=1`: a single craft drawn live by the
  engine's renderer from a camera of the UI's, in place of its wireframe (needs
  render3d 0.7.0; without it the wireframe stays).
- `[HUD] InfoScale` (1.3): the info panel's size.

Confirmed in game 2026-10-17.

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

Confirmed in game 2026-10-17.

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

Confirmed in game 2026-10-17.

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

Confirmed in game 2026-10-17.

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

Confirmed in game 2026-10-17.
