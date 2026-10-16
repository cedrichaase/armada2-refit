# The HUD, a panel at a time

How `UI.asi` takes over the in-mission HUD (`PLAN.md`, "HUD: the stock panels keep
running; we draw them and route their input"): what the HUD is made of, the one switch
that hands a panel to us, the engine reads each panel needs, and the panels done so
far. Addresses are this build's `Armada2.exe` (GOG, patch 1.1), named from
`armada2.map`, and live only in `src/engine/`; each is checked against stock before
anything is patched or called.

    [HUD]
    Enable=1      ; 0: the HUD is stock whatever the panel keys say
    Bar=0         ; the button bar is ours (below)
    BarKeys=1     ; with it, the grid's keys; 0 keeps stock's bar keys
    BarLabels=1   ; the key in each button's corner
    Minimap=0     ; the minimap is ours (below); MinimapSize=300, its side in units
    Selection=0   ; the selection overlay is ours; SelectionGauges=1 shows the
                  ; selected units' gauges always (0: as stock, while pointed at)
    UnitView=0    ; our frame around the unit view, which the engine still draws

Every panel key defaults to 0: with the shipped `UI.ini` the HUD is stock and the
panel switch is not even patched.

## What the HUD is made of

The in-mission HUD is a list of `DisplayInterface` components. Each is a global object
that adds itself to one or more views' simulate and render lists when it is
constructed (`AddView`), reads its rectangles from the GUI config in `PostLoad`
(`DisplayInterface::LoadRectangle`, hud/README.md), and then runs three separate
steps every frame:

- **simulate** (`DisplayInterface::SimulateAll` from `Program::DisplayInputProcess`):
  `PreSimulate`, then `GenericSimulate`, which calls `AlwaysSimulate` /
  `AlwaysGameSimulate`, the hidden or visible variant, and `FocusSimulate` /
  `FocusGameSimulate` **only for the component that owns the mouse**. A panel works out
  what it shows in its always-step (the bar's `Update`, the info panel's
  `Simulate*Object`), and reads clicks in its focus-step;
- **the mouse**: which component owns it is decided once a frame by
  `CursorInterface::RefreshOwner` (below). The owner's buttons alone see hover and
  clicks (`StandardButton::Simulate` compares `iCursor+0x24` with its panel), and the
  owner's `Render` names the cursor shape;
- **render** (`DisplayInterface::RenderAll` from `Armada_RenderFrame`): `Render` for a
  visible component, `HiddenRender` for a hidden one, in list order.

Every rectangle is in the 1600 x 1200 space the GUI config is converted into, which
the engine scales to the back buffer per axis (hud/README.md). A component's own
rectangle is at `+0x04` (left, top, right, bottom, inclusive), its hidden flag at
`+0x16`, its cursor at `+0x24`.

### The inventory

The render list of a Borg Instant Action at 1280x720, as `UI.log` lists it at the
start of each mission ("hud: 18 components, in drawing order"), with `HUD.asi`'s
canvas: `OverviewImp, SelectionDisplay, CinematicView, ShipDisplay, EditMenu,
MapRadar, ResourcePanel, ReplayPanel, PopupPalette, MessageDisplay, CommDisplay,
OverlayText, ObjectivesDisplay, DropPlayerBox, LoadPercentBox, PauseBox, ButtonPanel,
CursorInterface`. The chat lines and the text-entry line are not components: the
mission frame draws them itself after `RenderAll` (`ChatDisplay::Render`,
`TextInput_Update`; README.md, "The mission frame").

| Component (global) | What it shows | Its state comes from | Input | Drawn with | Takeover needs |
|---|---|---|---|---|---|
| `PopupPaletteImp` (`g_pPopupPalette`) — **the button bar, ours** | the selected units' commands, menus (orders, build, AI, formations, trade), build lists, special weapons | its `Update` (always-game-step): the selection and the menu open, into 21 `ControlButton`s | its buttons on click release (focus-step); its hotkey tables and menu toggles in `mProcessKeyboardInput` | each `ControlButton`: a sprite named by the command, tinted by state; labels for trade rates | done: `engine::palette` |
| `ShipDisplay` (`shipDisplay`) — the info panel | the selection: one unit (wireframe, name, class, captain, shields, hull, systems, crew, officers, energy), a builder's queue, or a group of icons | its `Simulate*Object` steps from the overview's selection; three heights (`infoPanelArea_0/_1/_2`) | clicks on group icons (select one), on queue items (cancel), on system icons (repair) — focus-step | `StandardBackground`, `StandardBar`, `StandardText`, `WireframeIcon`, `ShipIcon`, `BuildWireframe`, `RaceIcon`, `RepairIcon` | `engine::selection` (done: the handles); per object: name, class, health, shields, crew, officers, systems, build queue — next |
| `SelectionDisplay` (`selectionDisplay`) — **the selection overlay, ours** | brackets, health/shield/energy gauges, group numbers and names over units in the 3D view | the overview's selection, each object's state | none of its own (tooltips on the objects) | camera-projected 2D quads and text | done: `engine::overlay` |
| `MapRadar` (`mapRadar`) — **the minimap, ours** | the map, fog, units, the camera's view outline; ship/station filter buttons | `RadarComponent` from the scanner grids (fog) and the object lists | click/drag moves the camera or orders a move; filter buttons | the radar's own textures (`Gminimap`, `Gfog`, ...) and sprites | done: `engine::radar` |
| `ResourcePanel` (`g_resourcePanel`) | the resources (dilithium, metal, latinum, biomatter, crew, officers) at the top | `ResourceComponent`s asking `Team::GetResource` | tooltips only | `StandardBackground` and text | `engine::resources` (the amounts, done); which `eResource` is which |
| `ButtonPanel` (`g_buttonPanel`) | the top-right buttons: menu, comm, idle unit, view toggle, objectives, help, palette, more | fixed | its `StandardButton`s (focus-step) | `StandardButton` sprites with tooltips | each button's press (`OpenMenu`, `OpenComm`, `IdleShip`, `ToggleView`, `ShowObjectives`, `ToggleHelp`, `TogglePalette`, `ToggleShowMore`) |
| `CinematicView` (`g_cinematicView`) — the unit view (`cinematicPanelArea`) | a second 3D view of the action, or a video | its camera, the action camera's target | clicks on it | a second scene render inside `RenderAll` (render3d/README.md: the action camera is a second view) | keep the engine's render; ours draws the frame around it only (done, `UnitView`) |
| `MessageDisplay` (`g_messageDisplay`) | a mission message box (text and OK) | script calls (`SetText`) | its OK button; pauses the game | `StandardBackground`, `StandardText`, `StandardButton` | the text and the OK press (`mUnpauseCallback`) |
| `ObjectivesDisplay` (`g_objectivesDisplay`) | the briefing and objectives, ticks for completed ones | the mission's objectives file, `SetObjectiveComplete` | OK; keys while it has the mouse (`GameKeysProcessInput`) | as `MessageDisplay` | the objectives' text and completion (hud/README.md has where they are kept) |
| `CommDisplay` (`commDisplay`), with its `CommBar`s | multiplayer comms: players, teams, alliances, give resources/units, chat targets, allied victory, latency | the transport's `GameSetup` | many buttons, toggles and amount boxes | `StandardButton`/`ToggleButton`/text | `GameSetup` reads and the comm actions (`ProcessAlliance`, `GiveResource`, ...): late, with the menus' multiplayer screens |
| `PauseBox` (`pauseBox`) | "paused" and an unpause button | the game's pause state | unpause; it locks the mouse to itself (`AlwaysSimulate` writes the owner) | box, text, button | the unpause press |
| `DropPlayerBox` (`dropPlayerBox`) | a dropped player: drop or AI takeover, with a timer | the transport | its two buttons | box, text, buttons | multiplayer: late |
| `ReplayPanel` (`g_replayPanel`) | replay controls: restart, pause, slow, normal, fast | the replay | its buttons | `StandardButton`s | the five presses |
| `OverlayText` (`overlayText`) | timed messages in the middle of the screen (`AddMessage`) | script and game events | none | the HUD font | the message list |
| `LoadPercentageBox` (`loadPercentageBox`) | multiplayer load progress of the other players | the transport | none | box and text | with the loading screen |
| `EditMenu` (`g_editMenu`) | the map editor's menu; hidden in a game | — | — | — | not taken |
| `cOverViewImp` (`g_pOverView`) — the 3D view itself | selection boxes, the order cursor | the selection, the pointer's target | everything not on a panel: the owner of last resort | — | not a panel: stays the engine's |
| `CursorInterface` (`iCursor`) | the cursor shape and the tooltips (`DoTooltip`, `mDrawTooltip`) | the owner's cursor and `Tooltip` | owns the decision of who owns the mouse | the device's cursor; tooltip boxes in the HUD font | the cursor is PLAN.md's "not decided yet"; tooltips are ours per panel (the bar's are) |
| `ScreenFade` (`g_screenFade`) | fades to and from black | scripts | none | a full-screen quad | not taken (not in a game's list) |
| chat lines, text entry (not components) | chat, and the line being typed | `ChatDisplay`, `TextInput_*` | the keyboard while typing | the HUD font | text entry: PLAN.md's "not decided yet" |

### The order to take them over

1. **The button bar** (done): the logic is the stock bar's, `GridLayout.asi` already
   knew it, and every press goes through one function.
2. **The info panel (`ShipDisplay`)**: next, because the bar is placed against it, and
   it is the panel players read most. Its reads are per object (name, class, health,
   shields, crew, systems, the build queue), all from `engine::selection`'s objects;
   its clicks (group icons, the queue, system repair) each map to a game command.
3. **The resources** and **the top-right buttons**: few reads, few presses.
4. **Messages and objectives**, **the pause box**, **replay controls**: boxes with text
   and a button or two; they could share one screen spec.
5. **The selection overlay** (`SelectionDisplay`): needs the camera's projection.
6. **The minimap**: the most engine state (scanner grids, the radar's textures, the
   camera, the move order), last.
7. **Comms and the drop box**: multiplayer state, with the menus' multiplayer screens.

When every panel is ours, `HUD.asi`'s canvas edit, font and seams matter to nothing we
draw, and the install takes `HUD.asi` and `GridLayout.asi` out (PLAN.md).

## The panel switch

`engine/hud.rs`. Two walks over the component list make a panel visible and give it
the mouse; each is patched at the one instruction where it calls into a component,
by a `mov ecx, esi; call <ours>` in the same seven bytes, checked to be stock first.
No vtable is changed (`HUD.asi` recognises the bar by the `PostLoad` in its vtable).

| Site | What is there | What ours does |
|---|---|---|
| 0x51abf7, in `DisplayInterface::RenderAll` | the call of a visible component's `Render` (vtable +0x58) | skips a component we have taken over; and before `CursorInterface` renders, names the bar's arrow as the cursor when the pointer is over a panel of ours (below) |
| 0x505316, in `CursorInterface::RefreshOwner` | the call of each component's `CursorOver` (vtable +0x70) | says no for a component we have taken over, and no for every component while the pointer is over a panel of ours |

**Why there.** `RenderAll` is called once a frame, from `Armada_RenderFrame` only; its
loop calls `Render` for a visible component and `HiddenRender` for a hidden one, so a
hidden component of ours still gets its stock `HiddenRender` (the bar's clears its
state). `RefreshOwner` (in `CursorInterface::AlwaysSimulate`) is where the owner is
chosen: unless a component holds the lock (`iCursor+0x28`, a button while it is
pressed), the owner (`iCursor+0x24`) is the last visible component in the render list
whose `CursorOver` says the pointer is over it. A component that never becomes the
owner gets no focus-step, so none of its buttons sees a click or a hover, and its
tooltips never start. Its always-step still runs, so its state stays current.

**The pointer over a panel of ours.** The UI's input capture keeps the clicks from the
engine (README.md, "Input and capture"), but not the pointer, and with the stock
owner decision the 3D view would own it under our panel: hover highlights and order
cursors through the bar. So while the last UI frame claimed the pointer, no component
owns it. With no owner, `CursorInterface::Render` gives the device no cursor at all
(each owner's `Render` puts its cursor in `iCursor+0x2c` for that), so just before it
renders, ours puts the bar's own cursor there (`g_pPopupPalette+0x24`, which the bar's
`PostLoad` loads with `LoadCursor`): the same arrow a stock panel shows.

**Taking a panel.** `engine::hud::set_taken(at, by_pointer, on)`: a component by its
address, or (the bar) by the global that points to it, read when asked.

## The button bar

`hud/bar.rs` (the panel) over `engine/palette.rs` (the reads and the press), with the
grid of `grid/README.md`, which it supersedes: `[HUD] Bar=1` is the grid and its keys
without `GridLayout.asi`, and **stands down while `GridLayout.asi` is loaded** (the
two would wrap the same call; UI.log says "GridLayout.asi is loaded and owns the
bar").

**What it reads.** The stock bar's `Update` still decides the buttons: its 21
`ControlButton`s at `+0x28`, shown when `IsValid` (vtable +0x1c). Each button's kind
and name are grid/README.md's (`CommandInfoClass` at `+0x88`, else `ModeInfo` at
`+0x84`), its state at `+0x34` (0 disabled, 2 active: the alert level in force and
the like). The menu open is `+0x124`, the palette mode `+0x110`; the bar is hidden
with `+0x16`, and the whole HUD with the info panel's (`ingame::hud_hidden`).

**The cells** are grid.c's assignment, ported: cancel on T and back/close on G in every
menu; at the top level the build/research/evolve opener first in the top row, the other
menus after it, special weapons along the bottom, commands in the middle; in a submenu
each command at its `preferredPosition` (a three-column plan, so the AI menu is a 3x3);
everything else down the left columns. One trap: a press changes the menu (`+0x124`) at
once but the buttons only at the bar's next update, so for a frame the old buttons sit
under the new menu, and laid out by it they jump. The bar lays out by the menu its
buttons were built for (recorded in the bar's keyboard step, which runs right after
the buttons are built) whenever that step has run in the last quarter second.

**Icons.** `ControlButton::Render` looks its sprite up every time it draws: the
command's or mode's `ButtonName(std::string &)` names it (`CommandInfoClass` 0x519780,
`ModeInfo` 0x4e7950), and `StandardButton::SetImages` finds it by that name in the HUD's
sprite database (`DisplayInterface::m_spriteDatabase` 0x765030,
`ST3D_Database::FindElement` 0x620750). With the stock `Render` skipped nothing does
that, so `palette::sprite` does it the same way, cached per name while the database
is the same. The sprite is drawn by our renderer (`engine/sprite.rs`): an
`ST3D_Sprite` draws the texture at `+0x58` over the rectangle at `+0x38` (u, v, width,
height, 0..1, as `ST3D_Sprite::DrawScaled2D` hands them to the device), and a texture
is named (`+0x8`, as every database element) after its file in `Textures/RGB/`, which
`gfx::load_tga` loads, so an upscaled texture pack shows. Measured: the Borg icons are
256x256 files (`gbbuild00`, `gbforder00`, ...) drawn whole, and the shared ones are
quarters of a 128x128 (`b_stop` is `gbgenorders1` at 0, 0.5, 0.5, 1).

**Tooltips** are the game's own words: `ControlButton::GetTooltip` (vtable +0x00) and
`GetVerboseTooltip` (+0x04) write into a `std::stringstream` (the command's localised
text and hotkey; for a build item, its cost). `engine/msvc.rs` makes one through the
game's own `msvcp60.dll` imports (the constructor at the import slot 0x7b7c58 with
mode `in|out` and the "construct the virtual base" flag, as the game calls it;
`str()` at 0x7b7db0; the whole-object destructor at 0x7b7ea8), and frees it the same
way. The short text shows after 0.35 s, the verbose one after 1.5 s. The strings mark a
hotkey's letter with `^` and name stock's hotkey in `<...>`, which does nothing while
the grid's keys are on: both are dropped.

**A press** is what releasing a click does in `StandardButton::Simulate`: nothing if
the button is disabled, else the click sound (unless a video plays) and the button's
press function (vtable +0x20, `ControlButton::mButtonPressFunction`), which hands the
command (`CheckCanExecute`) or the mode to the bar. Clicks and keys are both pressed
in the bar's keyboard step (the call of `mProcessKeyboardInput` at 0x4fb56a in
`Update`, wrapped as GridLayout.asi wrapped it), so they act at the moment stock's
would, on buttons of this update. A click is queued by the UI frame and dropped if no
keyboard step takes it within three frames (the step runs only when something of the
player's is selected and the game is not paused). The stock click's lock of the cursor
while the button is held is not reproduced: ours never owns the mouse.

**Keys** are grid/README.md's, by scancode (`GetAsyncKeyState` on the virtual key
`MapVirtualKeyA` gives for each), nothing while chat has the keyboard
(`TextInput_IsActive`), Ctrl or Alt is held, or the game is not the foreground window.
With `BarKeys=1` stock's bar keys do nothing: the three hotkey tables' counts are
emptied for the stock call and put back, and the six menu toggles point at a zero.

**Sounds.** The rollover sound when the pointer enters an enabled button, the click
sound on a press: `AudioSound2D` objects made with the game's `operator new`, named by
`StandardButton::s_buttonRolloverSound` and `s_buttonClickSound`, as the stock button
plays them (`engine/sound.rs`).

**Where it goes.** At the bottom between the minimap and the info panel when the room
there is enough for it, else above the info panel, aligned with its left edge, moving
with the panel's height (low, middle, tall). Both rectangles are the components' own
(`+0x04`, scaled to the back buffer). At 1280x720 with `HUD.asi` there is no room
beside, and the bar sits above the info panel.

**Log.** Each layout once per change ("bar: menu=2 mode=1 | Q=bbase ... T=cancel
G=back", `(off)` for a disabled button), each press ("bar: click build", "bar: key
S=bmining").

### Seen on the bench

2026-10-16, Instant Action as the Borg at 1280x720 (`ui/bench-asi.sh --no-grid
HUD.Bar=1`): the bar drew as a 5x3 grid of the game's icons above the info panel, the
stock bar did not draw; a click on build opened the build list (13 items, cancel on T,
back on G), S started a mining station's placement and a click placed it; W opened
orders (W=repair E=superRepair A=patrol S=scout Z=guard X=decom G=back), R the AI menu
as a 3x3 with the active alert levels outlined, E formations; G came back each time; on
the Nexus, Q opened its build list, Q twice queued two construction ships and T
cancelled one. The selection stayed through every click on the bar, and the cursor
over it was the arrow. `testbench/scenarios/ui-hud-bar.md` is the regression test:
every step passed (judged and logged), but its last, `Quit the game`, which hangs on
a black screen after Exit to Windows on the bench that night with UI.asi taken out
too (`testbench/operating/hud-takeover.md`).

## The minimap

`hud/minimap.rs` (the panel) over `engine/radar.rs` (the reads, the clicks), with
`[HUD] Minimap=1`: a square of `MinimapSize` units (300, at most 45% of the screen's
height) at the bottom left, in a panel of the theme with the two filter buttons above
it. The stock `MapRadar` keeps running and is neither drawn nor given the mouse.

**The square.** `RadarComponent` places everything in a unit square: the map's box
(`g_terrain`: x0 y0 z0 x1 y1 z1) centred in a square `+0x5c` world units across, the
offsets `+0x60` / `+0x64` centring it, v running against z. Its
`ConvertWorldToSquareNormal` (0x4e8670) and `ConvertSquareNormalToWorld` (0x4e86e0,
which puts the point at the map's middle height) are called for every point, so ours
is placed exactly as stock's, at any size.

**What stock's `Render` draws, and where ours gets it** (stock's order, which ours
keeps):

| Layer | Stock | Ours |
|---|---|---|
| backing | `RadarComponent+0x3c`, a sprite | the theme's panel and a dark square |
| the map's picture | `m_mapSprite` (0x76387c), a 128x128 texture `CacheMap` (0x4e9e40) fills with every object's `UpdateMapObject` whenever `g_cachedMapNeedsUpdate` is set, from the panel's always-step | the same texture, drawn by our renderer through its Direct3D 9 texture (an `ST3D_Texture`'s per-device texture at `+0x40` + the device index times 4, its `+4` the `IDirect3DTexture8`, which d3d8to9 answers for its `IDirect3DTexture9`: render3d reads the fog's the same way); the stock `Render` is skipped, but the always-step that fills it runs |
| warp suppression | `m_warpSuppressionSprite` (0x763880), filled by `CacheMap` | the same, the same way |
| blips | `RenderMapObjects` (0x4ea4c0), twice: the sprite pass, then the dot pass | `radar::blips`, drawn by us (below) |
| fog, shroud | `m_fogSprite` / `m_shroudSprite`, written by `UpdateScanGridSprite` (0x4e96c0) from the always-step when the grid's dirty flag (`+0x18`) is set; drawn only when `GameSetup::isFog` / `isShroud` and the map editor is off (`UserProfile+0x84`) | masks of our own from the same grids (below), under the same conditions |
| pings | the `MessageMark` list at `+0x4c` (a position, a ring's radius in pixels at `+0x1c` that the always-step counts down, a colour at `+0x20`), each a ring (`ST3D_Camera::DrawCircle2D`) | rings of ours, the radius scaled from the stock radar's width to ours |
| the camera | `RenderCameraViewFrustum` (0x4ea760) | ours (below) |

**Blips** are `RenderMapObject`'s (0x4ea150) tests, kept: every object of
`GameObject::objectList` not being removed (`+0x113`) that `CanUserSee` (vtable
`+0x08`), at its position (`+0xac`) for its team (`+0xec`), then every `Spirit` of
`Ghost::ghostList` (last-known enemies under the fog: class `+0x4`, team `+0xc`,
position from its vtable `+0x10`), drawn faint. The class (`+0x40`) is left out when
it is hidden from the radar (`+0x18c`) or outside the map, and a craft class
(`IsCraftClass`, vtable `+0x28`) that is not a planet (`__RTDynamicCast` to
`PlanetClass`) passes the ship or station filter (`sg_shipFilterState` /
`sg_stationFilterState`; a station is `CraftClass+0x20d`): 0 all, 1 allies, 2
enemies, 3 neutral, by the user team's `Team::IsUnconfirmedAlly` / `Enemy` /
`Neutral`. A class with a radar sprite (`+0x190`: the moons, the planets) shows it; any
other a dot whose size is `ComputeDot`'s (0x4e99d0): `MapSize` (vtable `+0x24`) when the
class has one (`+0x184`), else 2 for a ship, 3 for a station, 1 for anything else, in
texels of a 128-texel map. Ours draws ships as round dots and stations as squares at
that size (never under 7 / 10 units), with a white ring round a selected unit
(`+0x10e`). The colour is the team's map colour (`Team::GetTeam(team)+0x14`, three
floats, which `Team::SetMapColor` sets); a team with none (black: the bench's scenes
make teams without a game setup) is coloured by its relation to the player instead
(own `good`, ally `accent2`, enemy `bad`, neutral `warn`).

**Fog and shroud.** `Scanner::m_fog` / `m_shroud` (0x738a54 / 0x738a58) are
`ScanGrid`s: `+0x4` the cells, one `u32` each, row by row, `+0xc` wide and `+0x10`
tall (128x128 on Warzone and on `a2_borg01`), each a mask of the teams that see (fog)
or have seen (shroud) the cell. A cell is clear when it shares a bit with
`Entity::GetTeamsVisibleToUser` (0x4d0070). Stock copies a pattern texel into its
sprite for every hidden cell, the grid centred in the sprite's texture (`+0x1c` /
`+0x20` its size; 128x128 too). Ours builds one A8R8G8B8 texture per layer at the
grid's own size, white and opaque where hidden, rebuilt only when the packed
visibility bits (or the team mask) changed, and draws it tinted over the grid's
rectangle in the square: the shroud opaque in the slate of the 3D view's shroud, the
fog in the same slate at 60%. Grid row 0 is the square's top (seen on the bench: the clearing
follows a ship sent north). The engine's textures are never resized or replaced
(CLAUDE.md: `UImid` froze the game).

**The camera.** In the RTS view (`View_Record` 0) stock casts the 3D view's corners
onto the map's middle height and joins them. Ours casts the main camera's viewport
corners (`ST3D_GraphicsEngine+0xfc`, the camera in use outside
`CinematicView::Render`, which puts it back; its viewport `+0x1b0`, near `+0x1a0`, far
`+0x1a4`, position `+0xe4`) with `ST3D_Camera::ProjectScreenPointToWorldRay`
(0x619140: the distance along the ray to that height, and the ray), the distance taken
as the far plane's when it is not between the near and far planes, and draws the four
sides. In the tactical view (1) stock draws a ring about the point the camera turns
about, through its eye (`gTacticalCamera+0x98` / `+0xa0` and `+0x8c` / `+0x94`, x and
z), and ours does too. The shot director's grey rings in that view are not drawn.

**Clicks** are `RadarComponent::Simulate`'s (0x4e8a90), which stock calls from the
panel's focus-step, at the point under the pointer in the world. With something
selected (the overview's `GetSelectNum`) the command button (the right; the left when
`UserProfile+0x24` swaps the buttons) asks `ActionMode::GetAction` (0x44ab70: no
target, the point) what a click there means; a plain move (`& 0xfff` = 4) without Alt
(`g_pCommandAlt`) becomes 0x2b, keeping the queue bit 0x4000, at the height of the
cursor (`cOverViewImp::GetWhereCursor`, vtable `+0xbc`); the overview is told
(`SetCursorAiAction`, `+0xc8`) and the bar sends it (`PopupPaletteImp::BroadcastCommand`,
vtable `+0xa4`), as a right click in the 3D view does. The other button held, or either
with nothing selected, moves the camera there (`gCameraManager`'s first field, the
camera in use, vtable `+0x54`); in the tactical view a click moves it once and
`CameraManager::SnapToNearestVisible` (0x4de460, with `cfgTACTICAL_SNAP_DISTANCE`)
snaps it, unless Alt is held. Ours queues the point from the UI frame and makes the
call from the panel's always-step: `MapRadar::AlwaysGameSimulate`'s call of
`Entity::GetTeamsVisibleToUser` (0x4eb3b2) is re-pointed at ours, which calls it and
then acts, so the order is given in the game's simulate as stock's is. A point not
taken within half a second (the game paused) is dropped. `UI.log`: "minimap: order
0x2b at x,z".

**The filter buttons** press the stock buttons' functions, `MapRadar::NextShipFilter`
(0x4eb7c0) and `NextStationFilter` (0x4eb930) (static; they cycle the state and post
its name in the chat, "All ships visible" and so on), from the same step. Each shows
the stock button's art (`StandardButton+0x2c`: `mapfilter_ship_button` in
`uiFilterButtons`), a coloured mark and a word for the state, and as tooltip the game's
own strings (`Localized_Strings::Localize` on `g_string`: `GUI_MM_SHIPS_TOOLTIP`,
`GUI_MM_ALL_SHIPS_VISIBLE`, ...).

**HUD.asi** re-lays the stock `MapRadar` out after a display mode change
(hud/README.md, "Re-layout"); ours reads no stock rectangle but the stock radar's
width, to scale a ping, so that keeps working. The button bar (`hud/bar.rs`) still
places itself by the stock radar's rectangle (`MapRadar+0x04`), which ours covers at
the default size.

## The selection overlay

`hud/overlay.rs` over `engine/overlay.rs`, with `[HUD] Selection=1`. The stock
`SelectionDisplay` keeps its always-step (the unit under the pointer, the gauges' fade,
`SimulateSelectionEffects`) and its `PreRender` (the 3D selection bubbles, which the
engine or render3d draw): only its `Render` is skipped.

**What stock draws** (`SelectionDisplay::Render`, 0x509540): for each object of
`GameObject::objectList` not being removed (`+0x113`), drawn this frame (`+0x25`) and
seen (`CanUserSee`), and not a station (`CraftClass+0x20d`) unless it is under the
pointer: the group number (`mDrawGroups`: `+0x120`, 0..9, -1 none) when it is selected
(`+0x10e`) or pointed at (`selectionDisplay+0x7c`, which the always-step sets from
`DisplayInterface::MouseOn` while the 3D view owns the mouse); and the gauges when it
is pointed at or of the pointed unit's team (`+0x88`, the pointed unit's `+0xec`), once
`+0x8c` has faded in above 0.5. The gauges (for a craft, `GameObject+0x14` bit 3): the
hull `+0x158` (a fraction, while the maximum `+0x160` is above 0.1), the shields
`Craft+0x1c8` of `+0x1cc`, the special energy `+0x164` (while `+0x168` is above 0.1),
and the damaged systems' icons (`RepairIcon`s at `+0x54`). Stock draws no brackets: the
selection is the 3D bubble.

**Where.** `SelectionDisplay::ComputeContainerRect` (0x5081b0, called) projects the
eight corners of the object's bounding box (`GetBoundingBox`, vtable `+0x1c`) through
`DisplayInterface::currentCamera` (0x765010) with `ST3D_Camera::ProjectVirtualPoint`
and returns their box, `{x, y, w, h}`, in the virtual viewport
(`DisplayInterface::s_pVirtualViewport` 0x765004, its size at `+0x8` / `+0xc`), which
ours scales to the back buffer per axis. Stock's `Render` points `currentCamera` at
`ST3D_GraphicsEngine+0xfc` first; ours does the same around its calls and puts back
what was there. So the brackets sit where the engine's gauges sit, by the engine's own
projection, whatever renders the scene; UI.asi does not depend on render3d.

**Ours** draws, for every selected unit and the pointed one: four corner brackets in
the theme's accent (an ally `good`, an enemy `bad`, neutral `warn`; lighter when
pointed at), at least 26 units across; above them the hull gauge (`good` above 60%,
`warn` above 30%, else `bad`), then the shields and the special energy as thinner
gauges; the group number in a small box at the left; and for the pointed unit its
class's name (`GetClassDisplayName`, vtable `+0x70`; `+0x48`, which `mDrawLabel` would
print, is the object's script label, "bconst1051", not for the player). With
`SelectionGauges=1` (the default) the selected units always show their gauges; 0
shows them as stock does. A unit whose middle is under a stock panel still drawn, or
under our minimap, is left out, and the overlay draws before our other panels, so
they cover it.

## The unit view

`hud/unitview.rs`, with `[HUD] UnitView=1`. `CinematicView` (`g_cinematicView`,
0x763758) is a second render of the scene with its own camera
(`ST3D_GraphicsEngine::SetCurrentCamera` around `RenderView`, which calls
`Armada_RenderAllOurStuff`: render3d/README.md, the action camera) or a Bink video:
only the engine can draw it, so it is not taken over. Ours draws a frame of the theme
just outside its rectangle (`+0x04`), with the form's accent at two corners, while it
is shown. `DisplayInterface::SetInterfaceState` shows it in state 0 and hides it in the
others.

### Seen on the bench (minimap, overlay)

2026-10-16. Instant Action as the Borg at 1280x720 (`ui-hud-map`): our minimap drew at
the bottom left with the base's clearing, its blips and the camera's outline; a right
click on it ordered the selected Assembler there ("minimap: order 0x2b"), which moved;
a left click moved the camera; a ship sent north carried its clearing with it (grid
orientation). The selected Assembler showed brackets and its hull and shield gauges.
`fog` scene at 1920x1080 (`ui-hud-overlay`): three ships in group 1 showed brackets,
"1" badges and gauges, the destroyer under Klingon fire with a short red hull gauge;
the minimap showed own blips green, the enemy red, and the tactical view's ring. The
judged steps passed in earlier runs; the last runs (after the fog colour change)
passed every logged step, their judged steps left for review (the judge was
unavailable). `UnitView=1` was never seen drawing: the unit view stayed hidden.

## The engine API

One file per area in `src/engine/`, each read by offset through small accessors, each
named here beside the panel that uses it:

| File | What | Used by |
|---|---|---|
| `hud.rs` | the components (globals, rectangles, hidden, names), the panel switch, `typing`, `video_playing` | every panel |
| `palette.rs` | the bar: its buttons, kinds, names, states, icons, tooltips, the press, the keyboard step | the bar |
| `sprite.rs` | a game sprite's texture and rectangle, loaded for our renderer | the bar; every panel that shows game art |
| `msvc.rs` | `std::stringstream` and `std::string` through the game's `msvcp60.dll` | tooltips, names |
| `sound.rs` | the HUD's button sounds | every panel with buttons |
| `selection.rs` | the selected objects' handles (the overview's `GetSelectList`, vtable +0x98, and count, +0x94: `QOL.asi`'s bigger selection reads the same way), `Entity::Get`, the user's team | the info panel, the selection overlay |
| `resources.rs` | `Team::GetResource` for the user's team | the resource panel |
| `radar.rs` | the radar's square, its picture's textures, the scan grids, blips, pings, the camera's outline, team colours and relations, the filters, the move order and the camera move (the always-step hook), `Localize` | the minimap; relations for the selection overlay |
| `overlay.rs` | the units `SelectionDisplay` marks, their boxes on screen (`ComputeContainerRect`), gauges, groups, names; `g_cinematicView` | the selection overlay, the unit view's frame |

## Not done

- **The info panel** and every other panel (the order above). For the info panel the
  likely way in is the panel's own child components rather than the objects: its texts
  are `StandardText` subclasses (`NameText`, `ClassText`, `CrewText`, `HullText`, ...)
  whose `GetInformation` answers the colour and the string the stock panel draws, so
  ours would show exactly what stock computes; the bars, the wireframe and the icons
  need their own reads, and its clicks (group icons, the build queue, system repair)
  their own presses.
- `GridLayout.ini`'s `[Cells]` (moving one button to another key) has no `UI.ini`
  counterpart yet; nor the trade buttons' exchange-rate label and the percentage some
  commands draw on their button (`ControlButton::Render` draws both as text).
- A stock click on a palette button locks the cursor to the bar while held, and the
  bar's `PreSimulate` sends the bar back to the top level when a click lands outside it
  while it holds the lock; ours never holds it. Not seen to matter.
- Only the Borg have been seen, at 1280x720.
- **Minimap / overlay**: the damaged systems' icons over a pointed unit, the shot
  director's rings in the tactical view and the pointer's action cursor over the
  minimap (ours shows the arrow) are not drawn; `UnitView` is unseen; the bar still
  places itself by the stock radar's rectangle.
