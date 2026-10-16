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
| `SelectionDisplay` (`selectionDisplay`) | brackets, health/shield/energy gauges, group numbers and names over units in the 3D view | the overview's selection, each object's state | none of its own (tooltips on the objects) | camera-projected 2D quads and text | selection + per-object gauges + the camera's projection; draws over the 3D view, so last |
| `MapRadar` (`mapRadar`) — the minimap | the map, fog, units, the camera's view outline; ship/station filter buttons | `RadarComponent` from the scanner grids (fog) and the object lists | click/drag moves the camera or orders a move; filter buttons | the radar's own textures (`Gminimap`, `Gfog`, ...) and sprites | the minimap picture (its texture, or ours), the camera's position, the move order: last |
| `ResourcePanel` (`g_resourcePanel`) | the resources (dilithium, metal, latinum, biomatter, crew, officers) at the top | `ResourceComponent`s asking `Team::GetResource` | tooltips only | `StandardBackground` and text | done: `engine::topbar`, `hud/resources.rs` |
| `ButtonPanel` (`g_buttonPanel`) | the top-right buttons: menu, comm, idle unit, view toggle, objectives, help, palette, more | fixed | its `StandardButton`s (focus-step) | `StandardButton` sprites with tooltips | done: `engine::topbar`, `hud/buttons.rs` |
| `CinematicView` (`g_cinematicView`) — the unit view (`cinematicPanelArea`) | a second 3D view of the action, or a video | its camera, the action camera's target | clicks on it | a second scene render inside `RenderAll` (render3d/README.md: the action camera is a second view) | keep the engine's render; ours draws the frame around it only |
| `MessageDisplay` (`g_messageDisplay`) | a mission message box (text and OK) | script calls (`SetText`) | its OK button; pauses the game | `StandardBackground`, `StandardText`, `StandardButton` | done: `engine::popups`, `hud/messages.rs` |
| `ObjectivesDisplay` (`g_objectivesDisplay`) | the briefing and objectives, ticks for completed ones | the mission's objectives file, `SetObjectiveComplete` | OK; keys while it has the mouse (`GameKeysProcessInput`) | as `MessageDisplay` | done: `engine::popups`, `hud/objectives.rs` |
| `CommDisplay` (`commDisplay`), with its `CommBar`s | multiplayer comms: players, teams, alliances, give resources/units, chat targets, allied victory, latency | the transport's `GameSetup` | many buttons, toggles and amount boxes | `StandardButton`/`ToggleButton`/text | `GameSetup` reads and the comm actions (`ProcessAlliance`, `GiveResource`, ...): late, with the menus' multiplayer screens |
| `PauseBox` (`pauseBox`) | "paused" and an unpause button | the game's pause state | unpause; it locks the mouse to itself (`AlwaysSimulate` writes the owner) | box, text, button | done: `engine::popups`, `hud/pause.rs` |
| `DropPlayerBox` (`dropPlayerBox`) | a dropped player: drop or AI takeover, with a timer | the transport | its two buttons | box, text, buttons | multiplayer: late |
| `ReplayPanel` (`g_replayPanel`) | replay controls: restart, pause, slow, normal, fast | the replay | its buttons | `StandardButton`s | done: `engine::popups`, `hud/replay.rs` (not seen: no replay on the bench) |
| `OverlayText` (`overlayText`) | timed messages in the middle of the screen (`AddMessage`) | script and game events | none | the HUD font | done: `engine::overlay`, `hud/overlay.rs` |
| `LoadPercentageBox` (`loadPercentageBox`) | multiplayer load progress of the other players | the transport | none | box and text | with the loading screen |
| `EditMenu` (`g_editMenu`) | the map editor's menu; hidden in a game | — | — | — | not taken |
| `cOverViewImp` (`g_pOverView`) — the 3D view itself | selection boxes, the order cursor | the selection, the pointer's target | everything not on a panel: the owner of last resort | — | not a panel: stays the engine's |
| `CursorInterface` (`iCursor`) | the cursor shape and the tooltips (`DoTooltip`, `mDrawTooltip`) | the owner's cursor and `Tooltip` | owns the decision of who owns the mouse | the device's cursor; tooltip boxes in the HUD font | the cursor is PLAN.md's "not decided yet"; tooltips are ours per panel (the bar's are) |
| `ScreenFade` (`g_screenFade`) | fades to and from black | scripts | none | a full-screen quad | not taken (not in a game's list) |
| chat lines, text entry (not components) | chat, and the line being typed | `ChatDisplay`, `TextInput_*` | the keyboard while typing | the HUD font | done: `engine::chat`, `hud/chat.rs` (drawn by us, the keys the engine's) |

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
| `buttons.rs` | a `StandardButton`'s picture, state, label, tooltips and press; presses queued for the simulate pass; an `ObjectivesText`'s string and colour; `Localize`; Latin-1 strings | the top-right buttons, the boxes, the replay controls |
| `topbar.rs` | `ResourcePanel`'s components (which `eResource` the race shows), the amounts as stock writes them, the resources' colours and tooltips; `ButtonPanel`'s eight buttons and "show more" | the resources, the top-right buttons |
| `popups.rs` | `MessageDisplay`, `ObjectivesDisplay`, `PauseBox`, `ReplayPanel`: shown, texts, objectives and their completion, their buttons | the boxes, the replay controls |
| `overlay.rs` | `OverlayText`'s messages (point, justification, text, colour, fade) | the overlay messages |
| `chat.rs` | the chat lines, the text-entry regions, the skip of both stock drawings | the chat |

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

### Not done here

- **`CommDisplay` / `CommBar`, `DropPlayerBox`, `LoadPercentageBox`** (multiplayer).
  `CommDisplay` is a panel of `CommBar`s, one per player slot, each made by
  `CommBar::CreateButton`/`CreateToggle`/`CreateColorToggle`/`CreateAlliance` with
  callbacks (`PlayerSelectCallback`, `MakeEnemyCallback`, `MakeNeutralCallback`,
  `MakeAllyCallback`, ...) that a panel of ours would press as it presses a
  `StandardButton` (toggles and colour buttons are subclasses: their press slot is
  not `StandardButton`'s, so `engine::buttons::press` refuses them and they need
  their own check), and it reads `GameSetup` through `g_transport` for the names,
  teams and alliances. `DropPlayerBox` (`dropPlayerBox`, 0x764cd0) is a box with a
  `DropPlayerText` and two buttons, shown by the transport when a player drops;
  `LoadPercentageBox` (`loadPercentageBox`, 0x764de0) only text. Each needs a
  network game on the bench (`online/`'s two-player scenarios) to verify.

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
