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
    Info=0        ; the info panel is ours (below)
    Minimap=0     ; the minimap is ours (below); MinimapSize=300, its side in units
    Selection=0   ; the selection overlay is ours; SelectionGauges=1 shows the
                  ; selected units' gauges always (0: as stock, while pointed at)
    UnitView=0    ; our frame around the unit view, which the engine still draws
    Comm=0        ; the comm panel, DropPlayer=0 the dropped-player box, LoadWait=0
                  ; the wait for the others' loads (below, "Comms, ...")

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
| `ShipDisplay` (`shipDisplay`) — **the info panel, ours** | the selection: one unit (wireframe, name, class, captain, shields, hull, systems, crew, officers, energy), a builder's queue, or a group of icons | its `Simulate*Object` steps from the overview's selection; three heights (`infoPanelArea_0/_1/_2`) | a click on the one-unit face (camera to it), on group icons (select one; Shift: take it out), on queue items (cancel) — focus-step; system icons only have tooltips | `StandardBackground`, `StandardBar`, `StandardText`, `WireframeIcon`, `ShipIcon`, `BuildWireframe`, `RaceIcon`, `RepairIcon` | done: `engine::info` |
| `SelectionDisplay` (`selectionDisplay`) — **the selection overlay, ours** | brackets, health/shield/energy gauges, group numbers and names over units in the 3D view | the overview's selection, each object's state | none of its own (tooltips on the objects) | camera-projected 2D quads and text | done: `engine::overlay` |
| `MapRadar` (`mapRadar`) — **the minimap, ours** | the map, fog, units, the camera's view outline; ship/station filter buttons | `RadarComponent` from the scanner grids (fog) and the object lists | click/drag moves the camera or orders a move; filter buttons | the radar's own textures (`Gminimap`, `Gfog`, ...) and sprites | done: `engine::radar` |
| `ResourcePanel` (`g_resourcePanel`) | the resources (dilithium, metal, latinum, biomatter, crew, officers) at the top | `ResourceComponent`s asking `Team::GetResource` | tooltips only | `StandardBackground` and text | `engine::resources` (the amounts, done); which `eResource` is which |
| `ButtonPanel` (`g_buttonPanel`) | the top-right buttons: menu, comm, idle unit, view toggle, objectives, help, palette, more | fixed | its `StandardButton`s (focus-step) | `StandardButton` sprites with tooltips | done: `engine::topbar`, `hud/buttons.rs` |
| `CinematicView` (`g_cinematicView`) — the unit view (`cinematicPanelArea`) | a second 3D view of the action, or a video | its camera, the action camera's target | clicks on it | a second scene render inside `RenderAll` (render3d/README.md: the action camera is a second view) | keep the engine's render; ours draws the frame around it only (done, `UnitView`) |
| `MessageDisplay` (`g_messageDisplay`) | a mission message box (text and OK) | script calls (`SetText`) | its OK button; pauses the game | `StandardBackground`, `StandardText`, `StandardButton` | done: `engine::popups`, `hud/messages.rs` |
| `ObjectivesDisplay` (`g_objectivesDisplay`) | the briefing and objectives, ticks for completed ones | the mission's objectives file, `SetObjectiveComplete` | OK; keys while it has the mouse (`GameKeysProcessInput`) | as `MessageDisplay` | done: `engine::popups`, `hud/objectives.rs` |
| `CommDisplay` (`commDisplay`), with its `CommBar`s | multiplayer comms: players, teams, alliances, give resources/units, chat targets, allied victory, latency | the transport's `GameSetup` | many buttons, toggles and amount boxes | `StandardButton`/`ToggleButton`/text | done: `engine::comm`, `hud/comm.rs` |
| `PauseBox` (`pauseBox`) | "paused" and an unpause button | the game's pause state | unpause; it locks the mouse to itself (`AlwaysSimulate` writes the owner) | box, text, button | done: `engine::popups`, `hud/pause.rs` |
| `DropPlayerBox` (`dropPlayerBox`) | a dropped player: drop or AI takeover, with a timer | the transport | its two buttons | box, text, buttons | done: `engine::comm`, `hud/comm.rs` |
| `ReplayPanel` (`g_replayPanel`) | replay controls: restart, pause, slow, normal, fast | the replay | its buttons | `StandardButton`s | done: `engine::popups`, `hud/replay.rs` (not seen: no replay on the bench) |
| `OverlayText` (`overlayText`) | timed messages in the middle of the screen (`AddMessage`) | script and game events | none | the HUD font | done: `engine::overlay_text`, `hud/overlay_text.rs` |
| `LoadPercentageBox` (`loadPercentageBox`) | multiplayer load progress of the other players | the transport | none | box and text | done: `engine::comm`, `hud/comm.rs` (after the load; the loading screen covers the load itself) |
| `EditMenu` (`g_editMenu`) | the map editor's menu; hidden in a game | — | — | — | not taken |
| `cOverViewImp` (`g_pOverView`) — the 3D view itself | selection boxes, the order cursor | the selection, the pointer's target | everything not on a panel: the owner of last resort | — | not a panel: stays the engine's |
| `CursorInterface` (`iCursor`) | the cursor shape and the tooltips (`DoTooltip`, `mDrawTooltip`) | the owner's cursor and `Tooltip` | owns the decision of who owns the mouse | the device's cursor; tooltip boxes in the HUD font | the cursor is PLAN.md's "not decided yet"; tooltips are ours per panel (the bar's are) |
| `ScreenFade` (`g_screenFade`) | fades to and from black | scripts | none | a full-screen quad | not taken (not in a game's list) |
| chat lines, text entry (not components) | chat, and the line being typed | `ChatDisplay`, `TextInput_*` | the keyboard while typing | the HUD font | done: `engine::chat`, `hud/chat.rs` (drawn by us, the keys the engine's) |

### The order to take them over

1. **The button bar** (done): the logic is the stock bar's, `GridLayout.asi` already
   knew it, and every press goes through one function.
2. **The info panel (`ShipDisplay`)** (done): the bar is placed against it, and it
   is the panel players read most.
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

## The info panel

`hud/info.rs` (the panel) over `engine/info.rs` (the reads and presses):
`[HUD] Info=1`. The stock `ShipDisplay` keeps running and is neither drawn nor given
the mouse (`set_taken` on `shipDisplay`); ours draws at the bottom centre, laid out by
its content, in the player's theme. The bar is placed against ours (`hud::info::rect`).

**The face.** Stock's `Render` and `FocusSimulate` both start with
`ShipDisplay::DetermineSelectedDisplayType` (0x4f2ee0), which reads the overview's
selection and sets the panel's object (`+0x1e8`) and height (`+0x1f4`). Ours calls it
once a frame: 0 nothing, 1 one object, 2 a builder (an object with flag `0x200`), 3
and 4 a group (up to 8, more). Nothing else of the panel's `Render` is needed: it only
draws. Its `AlwaysGameSimulate` (the repair icons' and wireframe's flash) and
`AlwaysSimulate` (the craft under the pointer, below) still run.

**The children.** `ShipDisplay::PostLoad` builds the panel's parts once and keeps them
as members; each is a `StandardComponent` whose vtable has `GetTooltip` (+0x00) and
`GetVerboseTooltip` (+0x04) writing into a `std::stringstream`. Stock's `Render`
hands each the object (a text's `SetCurrentObject`, +0x20; a bar's `+0x428`; an icon's
`+0x28`) and draws it. Ours hands it the object the same way and asks for its answer
instead of its picture: a text's `GetInformation` (+0x24) gives the colour (three
floats) and string stock prints, `GetLabel` (+0x28) a key of the localised strings
(`Localized_Strings::Localize` on `g_string`); a `TextAndSprite` has its icon at
`+0x104`, tinted by `GetCurrentColor` (+0x2c); a bar's `GetBarValue` (+0x24) gives value
and maximum, `GetBarColor` (+0x20) its colour. All return through a pointer they are
given. The members ours uses:

| `shipDisplay+` | Child | Shown for |
|---|---|---|
| `0x70` | `SDInfoBar`: its `ShieldText` (+0xc) and `HullText` (+0x10), with icons | a craft's shield and hull rows; the hover card |
| `0x74`..`0x84`, `0xc4`..`0xd4` | five `SystemText`s and their `RepairIcon`s | a craft's systems (a planet: the first two) |
| `0x88`, `0x8c`, `0x98` | `CrewNumText`, `EnergyText`, `OfficerTextAndSprite` | crew above 0.1 (`+0x1c4`), special energy above 0.1 (`+0x168`), officers when the class costs any (`+0x88`) |
| `0x90`, `0x94` | `ClassText`, `NameText` | every object; the name for crafts and named objects |
| `0xa0` | `HealthBar` | a craft's hull; its colour is stock's shield reading (green to red, pure blue with the shields down) |
| `0xa4`, `0xb8` | `CargoBar`, `CargoAmtText` | a carrier's cargo; a resource's store (`+0x17c`) |
| `0xa8`, `0xbc`, `0xc0` | `PlanetaryMetalBar`, `PopulationText`, `PlanetOwnerText` | planets |
| `0xac` | `WireframeIcon` | its flash (+0x30) and tooltip |
| `0xb0`, `0xb4` | `ConstructionBar`, `ResearchItemText` | a producer or colony (build progress, what it builds); an object under construction |
| `0x120`..`0x144` | ten `BuildWireframe`s (builder +0x28, class +0x3c, slot +0x40) | the queue's tooltips |

Which parts an object gets is stock's choice, made in `info::single` as
`ShipDisplay::DisplaySingleObject` makes it, from the object's flags (`+0x14`: 0x8 a
craft, 0x20 a producer, 0x80 and 0x80000000 cargo, 0x4000 named, 0x40000 under
construction), `IsPlanet` (vtable +0x0c) and a `Colony` cast (`__RTDynamicCast`).

**The wireframe** is the game's art, read as `WireframeIcon::GetWireframe` finds it:
in the HUD's sprite database, by the class's wireframe name (`GameObjectClass+0x7c`;
for a `ConstructionObject`, the class it builds, `+0x188`), as `<name>w1`..`w5`, one
sprite per system, or `<name>_s` for an object drawn whole (flags `0x101000`, or a
planet). Each system's part is coloured by `Craft::mComputeRepairLeft` (0x4c8ca0) on
the craft's system (`Craft+0x1e0`, 0x30 bytes each): the theme's text when intact,
toward `bad` with the damage, `warn` on the flash while it is repaired (in the craft's
repair list, `Craft+0x30`, by the system's repair id 3, 2, 1, 5, 6, or a repair under
way, system `+0x28`); stock's are white, red and yellow. A whole one goes `bad` below
half health (`+0x158`). The gravity mine's slowing (`GameObject::GetSlowEffect`) and a
Romulan spy aboard (`GetLineOfSightEffect`, its ordnance class a `RomulanSpyClass`)
add stock's `gravmine_icon` and `romspy_icon`. The race emblem is `<race>_icon`, the
race's name at `GameObject+0xfc`, `+0x14`.

**The queue** is `Producer::GetBuildQueue` (0x4b8070): ten classes, 0 for a free slot.
A click on one is `Producer::RequestDeleteBuildQueueItem` (vtable +0x140) with the
slot, as `BuildWireframe::Simulate` does; the click sound with it.

**A group** is the selection (`engine::selection`, destroyed ones left out, `+0x113`),
each with its wireframe and hull. A click is `ShipIcon::Simulate`'s:
`cOverViewImp::Select` (vtable +0x90) with `(unit, 0, true, true)`, or with Shift
`(unit, 1, false, true)`, which takes it out. Shift is the game's own binding
(`g_pCommandShift`) or the keyboard's. There is no limit of 16: with `QOL.asi`'s
bigger selection every unit gets an icon.

**A click elsewhere on the panel** takes the camera to the object, as
`ShipDisplay::SimulateSingleObject` does on a click: the current camera of
`gCameraManager` (its first member), `SetInterestEntity` (vtable +0x58).

**The hover card.** `ShipDisplay::AlwaysSimulate` puts the craft under the pointer in
the 3D view at `+0x1ec`, and stock's `Render` draws `SDInfoBar` for it and clears it
(`Entity::NO_ENTITY`). Ours reads and clears it the same way, and shows that craft's
name, class, shields and hull in a small card beside the panel.

**Tooltips** are each child's own (`GetTooltip`, after 0.35 s; `GetVerboseTooltip`
after 1.5 s), drawn in the theme's tooltip as an overlay over the bar; a group icon's
is the unit's name and class. Stock's strings keep printf's `%%`: shown as `%`.

**Log.** Each object once ("info: Single USS Argo | Galaxy Class | 500 | 10 | hull
720/720 | shield 100% | ..."), each tooltip's text, each press ("info: cancel queue
slot 2", "info: select 0x...", "info: camera to 0x...").

### The player's race

`engine/race.rs`. `Race::s_missionRace` (0x737cc0) is the race the HUD is loaded in:
`DisplayInterface::PostLoadAll` sets it at the start of each mission, from the user's
team's race in the game setup, before it reads that race's GUI config and sprites.
Its name is at `+0x14` (`federation`, `borg`, ...). The mission frame hands it to
`theme::set_player_race` every frame, so `[UI] Theme=auto` draws the HUD in the race
whose stock HUD it replaces. The bar, too, now draws through `theme.rs` (its buttons
`button_frame`, its keys `badge`, its tooltip `tooltip`).

### Seen on the bench

2026-10-16, `-nointro a2_borg01` with the `ui-hud-info` scene at 1280x720
(`ui/bench-hud-info.sh`): the bench's HUD race came up Federation, Borg and Klingon on
different launches, and the theme followed each. One unit (the Galaxy: name, class,
hull and shield rows with stock's icons, crew 500, officers 10, five systems at 100%),
an enemy Negh'Var with its special energy, the dilithium moon (its store, 20000), the
starbase as a builder with "Construction Ship" and its queue (four queued through the
bar's keys, the third cancelled by a click, the dilithium refunded), a destroyer under
fire (wireframe and systems red, hull 10%, shields 0%), three destroyers as a group
(tooltips with name and class; a click selected one alone), the hover card for the
craft under the pointer, and stock's tooltips ("Hull Integrity at 100%"). A
Shift-click could not be tried: the bench's virtual keyboard does not reach the game's
Shift (a stock Shift-click in the 3D view does not add either).
`testbench/scenarios/ui-hud-info.md` is the regression test.
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
| `info.rs` | the info panel's face, its children's answers (texts, bars, tooltips), wireframes, systems' repair state, the queue, the group; select, cancel, camera | the info panel |
| `race.rs` | the race the HUD is loaded in | the theme |
| `radar.rs` | the radar's square, its picture's textures, the scan grids, blips, pings, the camera's outline, team colours and relations, the filters, the move order and the camera move (the always-step hook), `Localize` | the minimap; relations for the selection overlay |
| `overlay.rs` | the units `SelectionDisplay` marks, their boxes on screen (`ComputeContainerRect`), gauges, groups, names; `g_cinematicView` | the selection overlay, the unit view's frame |
| `buttons.rs` | a `StandardButton`'s picture, state, label, tooltips and press; presses queued for the simulate pass; an `ObjectivesText`'s string and colour; `Localize`; Latin-1 strings | the top-right buttons, the boxes, the replay controls |
| `topbar.rs` | `ResourcePanel`'s components (which `eResource` the race shows), the amounts as stock writes them, the resources' colours and tooltips; `ButtonPanel`'s eight buttons and "show more" | the resources, the top-right buttons |
| `popups.rs` | `MessageDisplay`, `ObjectivesDisplay`, `PauseBox`, `ReplayPanel`: shown, texts, objectives and their completion, their buttons | the boxes, the replay controls |
| `overlay_text.rs` | `OverlayText`'s messages (point, justification, text, colour, fade) | the overlay messages |
| `chat.rs` | the chat lines, the text-entry regions, the skip of both stock drawings | the chat |
| `comm.rs` | `CommDisplay` and its `CommBar`s, `DropPlayerBox`, `LoadPercentageBox`: texts, players, relations, chat switches, resource amounts, their buttons | the multiplayer boxes |

## The resources, the buttons, the boxes, the overlay and the chat

Each behind its own `[HUD]` key, all 0 by default: `Resources`, `Buttons`, `Messages`
(the message box and the objectives together), `Pause`, `Replay`, `Overlay`, `Chat`.
`hud/panels.rs` takes each over through the panel switch (by the component's
address: every one of them is a global instance) and draws them in one order: the
chat first, then the resources, the buttons and the replay controls, the overlay
messages over them (stock's `OverlayText` follows `ResourcePanel` in the list), and
the boxes that pause the game last, in the overlay layer over everything. While a box
shows, the screen is occluded for every other widget of ours and claimed from the
engine, as the stock boxes (whose `CursorOver` is always true) take the mouse
wherever it is; the keyboard is left to the engine. With the HUD hidden (a cinematic)
the resources and the buttons are not drawn.

**Presses at the simulate pass.** A stock `StandardButton` presses itself in its
panel's focus-step, inside `DisplayInterface::SimulateAll`; ours are clicked in the
UI's frame, which runs inside the engine's scene. A press there could open a dialog
(the menu button's `do_escape` runs the in-mission menu's modal loop) mid-scene. So a
click is queued (`engine::buttons::queue`) and pressed at the next pass of
`Program::DisplayInputProcess`, at its `call Entity::ResetVisibility` (0x483869),
which comes once per pass after the input loop that runs `SimulateAll` and before
`Armada_RenderFrame`: where a stock click would have pressed it, give or take the
order within the pass. The call is re-pointed, checked to be stock; without it,
presses are made in the UI's frame. A press not taken in half a second is dropped.
`press` checks the button's vtable slot is `StandardButton::mButtonPressFunction`,
and every panel checks each button's press function (`+0x5c`) is the one its
`PostLoad` set before it treats the field as that button.

### The resources (`ResourcePanel`)

`ResourcePanel::PostLoad` makes one `ResourceComponent` per `eResource` (0 crew, 1
officers, 2 dilithium, 3 latinum, 4 metal, 5 biomatter, `g_eResourceToString`) at
`+0x2c + 4 * r`, unless the race's GUI config sets `no<Name>`: so the panel's non-NULL
slots are the race's resources, and ours shows those, in that order.
`ResourceComponent::Render` writes the amount into its text each frame (and draws it
there): `Team::GetResource(r)` truncated, 0 while the team's `+0x16c` is clear, and for
officers the team's `+0x160` over `Team::GetMaxOfficers()` ("12/30"); ours writes the
same. The colour of each is `ResourceComponent::s_ResourceColor[r]` (a `ST3D_Colour *`
each, from the GUI config), drawn as a mark beside it; its name and tooltip are the
component's own `GetTooltip` (vtable `+0x00`, the verbose one `+0x04`), whose first
line is the localised name. A strip at the top centre of the screen.

### The top-right buttons (`ButtonPanel`)

Eight `StandardButton`s at `+0x2c..+0x48`: menu (`OpenMenu`, `do_escape`), comm
(`OpenComm`, shows `commDisplay`), idle unit (`IdleShip`: the overview's next idle
unit, and the camera to it), view (`ToggleView`, `CameraManager::SwapCameras`),
objectives (`ShowObjectives`), help (`ToggleHelp`, cycles
`CursorInterface::m_tooltipState` 0..2), palette (`TogglePalette`, the bar's vtable
`+0x90`), more (`ToggleShowMore`). `+0x4c` is "show more": `Render` and
`AlwaysSimulate` touch menu, help and palette only while it is set, and ours shows
them only then. Their states are `PostLoad`'s: comm is disabled outside a network
game, objectives when the profile says so, idle in some slot types. Ours: a row of
square buttons in the top-right corner, each drawing the stock button's own picture
(`+0x2c`, `engine::sprite`), the game's tooltip, pressing the stock button; help is
drawn active while the tooltip level is not 0, more while "show more" is set.

### Messages and objectives (`MessageDisplay`, `ObjectivesDisplay`)

`MessageDisplay`: the title (the message's first line) and the body are two
`ObjectivesText`s at `+0x28` and `+0x2c` (`SetText` splits the text at the first
newline), OK at `+0x34` (`mUnpauseCallback`: hides the box, and in a network game
clears `g_gameProcessObjectivesPause`). Its `CursorOver` is always true: stock's box
is modal. It reads no keys. `ObjectivesDisplay`: the summary's header
(`GUI_OD_BRIEFING_SUMMARY`) at `+0x28`, the mission's title `+0x2c`, the briefing
`+0x30` (seen on the bench: the title "Invasion", the briefing the long text), the objectives' header
(`GUI_OD_OBJECTIVES`) `+0x34`, the objectives a `std::vector<ObjectivesText *>` at
`+0x3c`/`+0x40`, their completion a vector of bytes at `+0x5c`/`+0x60`
(`SetObjectiveComplete`), OK at `+0x70` (`mUnpauseCallback`). Stock closes it from
its focus-step (`FocusSimulate`) when either of two command flags is set (pointers at
`s_completeCount+4` and `+8`, taken in its `PostLoad`; Enter is one, and the
tutorial's lessons advance on the same key). A component that never owns the mouse
never gets its focus-step, so with the box ours `popups::objectives_keys` calls
`ObjectivesDisplay::FocusSimulate` itself at each simulate pass while the box shows
(the presses' hook): the box closes on the engine's own bindings, as in stock.
Measured on the bench: a first version that pressed OK on Enter itself and kept the
key from the engine left the Starfleet Academy tutorial's lessons stuck and its
zoom unanswered; closed by its own step, the tutorial runs as with the stock box. Its
hidden-step, which opens it on the objectives key (Alt+O), still runs. Ours: windows
over a dimmed screen (`panels::boxed`; not `Ui::modal`, which keeps the keyboard),
the message's title as the window's, the text wrapped; the objectives with a check
box per line, ticked when complete.

### Pause and replay (`PauseBox`, `ReplayPanel`)

The pause box is shown and hidden by the pause order (`NetOrderPause::Execute`); its
unpause button at `+0x30` (`UnpauseCallback`) puts the unpause order into the round
buffer and hides it. Its text is a `PauseText` whose `GetInformation` answers no
string: the words are the GUI config's labels, `GUI_POP_PAUSE_TEXT` and
`GUI_POP_PAUSE_CONTINUE`, which ours localises itself
(`Localized_Strings::Localize` on `g_string`). Ours dims the screen lightly and keeps
the mouse (the game is paused), but not the keyboard, so the pause key still
unpauses. The box's `AlwaysSimulate` makes it the mouse's owner while the transport is
paused in view 2, over the panel switch's decision; not seen to matter.

The replay panel's five buttons at `+0x2c..+0x3c` press functions that cast the
transport to `TransportReplay` and set its speed (`+0x84`, 0 paused .. 3 fast) or go
to its first chapter. Ours: a row under the resources while the panel shows.

### Overlay messages (`OverlayText`)

`AddMessage(x, y, justify, text, colour, fade in, font)` keeps a `SingleOverlayText`
in a `std::map` at `+0x24` (head node `+0x28`; MSVC 6's tree with the shared `_Nil`
sentinel), keyed by a counter. Each message: its point at `+0x58` (1600 x 1200, from
the panel's corner), justification `+0x68`, text `+0x84` (a `std::string`), colour
`+0x94`, fade `+0xa0` (raised and lowered by the stock `AlwaysGameSimulate`, which also
erases a faded one: it keeps running). Ours draws each in title type with a shadow,
in its colour times its fade.

### Chat and the text entry (not components)

The mission frame calls `TextInput_Update` and then `ChatDisplay::Render` after
`RenderAll` (README.md, "The mission frame"). `ChatDisplay::Render` both drops the
expired lines and prints the rest: with `Chat=1` the UI's hook on its call
(`engine::ingame`) calls `ChatDisplay::RemoveExpiredText`, the same drop on its own,
instead. The call of `TextInput_Update` (0x598a07) is re-pointed at ours, which with
the chat taken only clears each region's "changed" flag, as stock does after
printing. The lines are a `std::list<ChatLine>` at `+0x04` (text, colour pointer);
a text-entry region keeps its prompt and text in one buffer (`+0x10`; the prompt's
length `+0x130`, the end `+0x134`, the caret `+0x138`) and flags at `+0x144` (1 shown,
0x100 taking keys). Ours: the lines down the left under the top bar on a faint
backing, and below them, while a region is shown, a field with the prompt in the
accent, the text and the caret. The keys are never ours: `TextInput_ProcessInput`
reads them from the engine's input as before.

### Seen on the bench

2026-10-16, 1280x720, the shell's theme (the race's theme is another agent's work):
Instant Action as the Borg showed Crew, Power Nodes ("16/600", the Borg's officers),
Dilithium and Metal, counting; the Federation campaign and the tutorial Crew,
Officers, Dilithium, Latinum and Metal. Every top-right button pressed its stock
function: more (menu, help and palette appeared), help (drawn active), idle (an
Assembler selected, the camera on it), view, palette (the bar hid and came back),
menu (the in-mission OPTIONS menu, Return to Game back), objectives (the box). The
game's tooltips showed, the verbose one after a rest. The pause key showed our box
("GAME PAUSED", "CONTINUE"), CONTINUE and the pause key both unpaused. Typing in chat
showed the line with the caret where the engine has it; the pause lines showed in
the chat. The Federation briefing and the tutorial's lesson boxes showed in ours,
closed on OK and on Return (by their own step), and the tutorial advanced on Return
and zoomed on the wheel as with the stock box. The intro's centre lines showed as
ours. `testbench/scenarios/ui-hud-panels.md` and `ui-hud-panels-objectives.md`.

## Comms, the drop box and the load wait

`hud/comm.rs` over `engine/comm.rs`, each behind its own `[HUD]` key, 0 by default:
`Comm` (the comm panel), `DropPlayer` (the dropped-player box), `LoadWait` (the wait
for the other players' loads). Each component is a global instance taken through the
panel switch, and each is read only after its vtable is checked to be its class's
(`CommDisplay` 0x6b5a5c, `CommBar` 0x6b5870, `DropPlayerBox` 0x6b5b68,
`LoadPercentageBox` 0x6b5d94), and each button only after its press function (`+0x5c`)
is checked to be the one its `PostLoad` gave it. Like the stock boxes, whose
`CursorOver` answers true wherever the pointer is, ours keep the mouse from everything
under them while they show (`comm::blocking`); the keyboard stays the engine's.

**Toggles are pressed through their own press.** The comm panel's chat switches, colour
buttons, allied victory and latency are `ToggleButton`s (or `ColorButton`s, built on
it), whose press slot (vtable `+0x20`) is `ToggleButton::mButtonPressFunction`
(0x510990): it flips the state between 1 and 2 and goes on to `StandardButton`'s, which
calls the callback; the callbacks read the new state. `engine::buttons::press` accepts
that slot as well as `StandardButton`'s, so a toggle is queued and pressed at the
simulate pass like any button. The `AllianceButton`s have `StandardButton`'s press.

### The comm panel (`CommDisplay`, `commDisplay` 0x764a60)

Opened by the top-right comm button (`ButtonPanel::OpenComm`, disabled outside a network
game) and closed by its OK (`OkayCallback`, `SetIsHidden(true)`). What `PostLoad` builds
and ours reads:

| What | Where | Notes |
|---|---|---|
| title, headings (player, message, team, alliance), group captions (message, give), allied-victory and latency captions | `StandardText *` at `+0x3c`..`+0x5c` | the text is the GUI config's label key at the text's `+0x54` (`GetLabel`), localised as `StandardText::Render` does |
| message all, allies, enemies, the selected player | `StandardButton *` `+0x60`, `+0x64`, `+0x68`, `+0x6c` | each sets which bars' chat switches are on (`CommBar::ActivateChat`) and calls the transport's `UpdateChatTargets` (vtable `+0x94`): the chat line typed next goes to them |
| give the selected units to the selected player | `+0x70` | `GiveUnitsCallback`: the overview's selection, sent as a transfer order |
| OK | `+0x74` | hides the panel |
| a resource row each: dilithium, metal, latinum, crew, biomatter | `+0x78`, `+0x7c`, `+0x80`, `+0x84`, `+0x88` | NULL for a resource the race has not (`Transfer*::UsedBy`); else 16 bytes: up, down, give, the amount's text. The buttons hold their `TransferDetails` at `+0x60`: the amount `+0x40`, least `+0x48`, most `+0x4c`, step `+0x50`. Up and down change the amount; give sends it (`ActionSend`). The resource's name is its `GUI_CP_*_RES` key (`Transfer*::GetName`) |
| allied victory, high latency | `ToggleButton *` `+0x8c`, `+0x90` | NULL when the game has none; on at state 2 |
| the bars | count `+0x94` (`commBarNumberOfPlayers`), `CommBar **` `+0x98` | one per slot, in slot order |
| teams fixed, alliances free | bytes `+0x9c`, `+0x9d` | from `GameSetup::getRelationshipType`: which of the team and alliance columns shows |
| the selected bar | `+0xa0` | `SetCurrentPlayer`; the player giving and the private message go to |

A `CommBar`: `+0x28` it stands for a player (cleared for an empty slot), `+0x2c` its slot,
`+0x38` its team shows, `+0x39` its alliance buttons work; the name button `+0x3c`
(`PlayerSelectCallback`, selects the player), the name text `+0x40` (a `PlayerNameText`,
the name inline at `+0x84`), the chat switch `+0x44` (`ChatToggleCallback`, which also
calls `UpdateChatTargets`; disabled for a computer), the team's text `+0x48` (a
`TeamNameText`: the team's number from 1 at `+0x84`, written after the localised
`GUI_POP_COMM_TEAM_PREFIX`), the enemy, neutral and ally `AllianceButton`s `+0x4c`,
`+0x50`, `+0x54` (`Make*Callback`: an alliance order; the relation in force has state 2,
`UpdateAlliance` sets it as orders arrive, and one proposed but not yet answered has
the byte `+0x7c` set, which the stock button draws as a second sprite), and the colour
button `+0x58` (a `ColorButton`, `PlayerSelectCallback` too; the player's colour, three
floats, at `+0x68`).

Ours: a window over the dimmed screen with the players' table (colour, name, team when
teams are fixed, the chat switch, enemy/neutral/ally when alliances are free, a proposed
relation framed in the warning colour), the selected player's row marked (a click on a
row presses its colour button), and beside it the message buttons, give units and the
resource rows (minus, the amount, plus, give); allied victory, latency and OK along the
bottom. Labels are the stock buttons' own or, for a picture button, its tooltip's first
line; English words only where neither exists.

### The dropped player (`DropPlayerBox`, `dropPlayerBox` 0x764cd0)

`Program::DisplayInputProcess` shows it while the transport waits on a player who stopped
answering and hides it when that is settled (`dropPlayerBox.SetIsHidden` in its log
lines), and keeps its seconds up to date (`UpdateTime`). `+0x2c`: the text, a
`DropPlayerText` whose `GetInformation` (vtable `+0x24`, returning the colour and the
text) writes the whole message, the wait and its seconds, who dropped and why, and the
votes cast; ours calls it and draws what it writes, the first line as the title.
`+0x30`: drop the player (`DropCallback`), `+0x34`: let the computer take over
(`AiTakeoverCallback`), each a vote through the network transport. Its
`AlwaysGameSimulate` (which keeps running) disables a button once its vote is cast, and
the computer's when the game type cannot have one.

### The load wait (`LoadPercentageBox`, `loadPercentageBox` 0x764de0)

Not part of the load: `Program::DisplayInputProcess` shows it once this player's load is
done (the mission's frames have started) while the transport still has load percentages
to show (`ShowLoadPercentages`, vtable `+0x40`), keeps the milliseconds waited in its
text (`LoadPercentageText`, `+0x284`, through `UpdateTime`), and hides it when every
player is in. So the loading screen of ours (`LOADING.md`), which shows the same
percentages during this player's load, and this box do not overlap: the box is the time
after. Ours: the stock title (`GUI_POP_LOAD_WAIT`), the seconds, and a row per network
player with a bar from the transport's `GetLoadPercentage` (`engine::setup::
load_fractions`, the loading screen's read); the stock text, from its `GetInformation`,
when the transport answers nothing.

## Not done

- Every panel after the info panel (the order above).
- The info panel: the planet face (population, owner, metal) has not been seen on the
  bench, nor a Shift-click on a group icon, the slowed and spy icons, or the
  verbose tooltips' wording at length. Stock's wireframe and system colours are
  re-made in the theme's status colours rather than copied.
- `GridLayout.ini`'s `[Cells]` (moving one button to another key) has no `UI.ini`
  counterpart yet; nor the trade buttons' exchange-rate label and the percentage some
  commands draw on their button (`ControlButton::Render` draws both as text).
- A stock click on a palette button locks the cursor to the bar while held, and the
  bar's `PreSimulate` sends the bar back to the top level when a click lands outside it
  while it holds the lock; ours never holds it. Not seen to matter.
- The bar: only the Borg had been seen, at 1280x720, before the info panel's runs (Federation, Borg, Klingon themes).
- Only the Borg have been seen, at 1280x720.
- **Minimap / overlay**: the damaged systems' icons over a pointed unit, the shot
  director's rings in the tactical view and the pointer's action cursor over the
  minimap (ours shows the arrow) are not drawn; `UnitView` is unseen; the bar still
  places itself by the stock radar's rectangle.
