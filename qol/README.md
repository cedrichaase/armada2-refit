# qol — gameplay quality of life

Changes to how Armada II *plays*, as opposed to how it looks: the controls, the camera,
control groups, production. Six of them are built in `QOL.asi`: the right-drag pan
speed (QOL-2), selections and control groups of up to 120 (QOL-3), Shift+number
adding to a group (QOL-4), stations in control groups (QOL-5), one build menu for
several stations, each order going to one of them (QOL-6), and long moves on the map
going to warp (QOL-8). The rest are planned here,
each with what is wrong today, what
we want, what is already known and what still has to be measured. What is installed, at which
version, and whether it has been seen in game is in [`CHANGELOG.md`](CHANGELOG.md).

    qol/install.sh                  build QOL.asi and install it with QOL.ini
    qol/install.sh --pan-speed 3    ... with PanSpeed=3
    qol/install.sh --remove         take it out (QOL.asi, QOL.ini, QOL.log)
    qol/rts-cfg-check.py [--fix]    is RTS_CFG.h stock where QOL.asi needs it to be?

**Where the log is.** `QOL.log` is in `%LOCALAPPDATA%\armada2-refit\logs\` (the Wine prefix's, under Proton), beside Display8's pipeline cache; the game directory only if that is unset or not writable. The first line says where. Written by `a2_log_path()` in `csdk/a2asi.h`; an uninstall leaves the folder.

`./install` runs `qol/install.sh`; `./a2mod stock` / `refit` switches `QOL.asi` and
`QOL.ini` like the other plugins.

## Two plugins: what other players need too

A change either stays on this player's machine or it changes the game every machine in a
network game simulates, and the two cannot ship in one plugin:

- **`QOL.asi` — stock-compatible.** The camera, the keys, which ordinary command a key
  press turns into. A player with it can join a player without it, and nothing they
  simulate differs. Installed by default. QOL-2 to 6 and QOL-8 belong here. QOL-1 became
  a plugin of its own, `GridLayout.asi` (retired), and is now the UI's bar with this plugin's keys.
- **A rules plugin (`QOLRules.asi`, the `rules/` layer since qol 2.0.0) — every player needs it.** Changes to
  what the game *does* with a command: when the bank is charged, what a group can
  hold if that travels over the network. Every node must run it with the same settings,
  so it needs a check when a network game is set up (the `online/` layer is the place)
  and it must refuse or stand down rather than desync. QOL-7 belongs here. (QOL-3 was
  the other candidate; bigger groups turned out not to change what is sent.)

The test for which side an idea falls on is the network game: would a stock player and
a player with the plugin, given the same orders, end up in the same game? If yes,
`QOL.asi`. If not, the rules plugin.

`RTS_CFG.h` is a trap on the compatible side: network games compare a CRC of its bytes
(below), so even a camera setting edited *in the file* locks a player out of games with
stock players. Compatible changes are made in memory and leave the file stock.

Status values: **idea** (not investigated), **scoped** (approach known, nothing built),
**in progress**, **done** (link to the layer).

| ID | Idea | Surface | First step | Status |
|---|---|---|---|---|
| [QOL-1](#qol-1-grid-hotkeys-for-the-button-bar) | Grid hotkeys for the button bar | `GridLayout.asi` (retired) | — | **done** ([`ui/HUD.md`](../ui/HUD.md)) |
| [QOL-2](#qol-2-configurable-right-drag-pan-speed) | Configurable right-drag pan speed | `QOL.asi`, `PanSpeed=` | A slider in the options screen | **done** (`QOL.asi`); slider open |
| [QOL-3](#qol-3-larger-control-groups) | Larger control groups | `QOL.asi`, `MaxSelection=` | A count on the selection panel; a stock peer in a network game | **done** (`QOL.asi`), bench |
| [QOL-4](#qol-4-shiftnumber-adds-to-a-group) | Shift+number adds to a group | `QOL.asi`, `ShiftAddsToGroup=` | — | **done** (`QOL.asi`), bench |
| [QOL-5](#qol-5-buildings-in-control-groups) | Buildings in control groups | `QOL.asi`, `StationGroups=` | — | **done** (`QOL.asi`), bench |
| [QOL-6](#qol-6-production-spread-across-a-group-of-buildings) | Production spread across a group of buildings | `QOL.asi`, `StationGroups=` | Construction ships | **done** for stations (`QOL.asi`), bench |
| [QOL-7](#qol-7-pay-when-queuing-refund-on-cancel) | Pay when queuing, refund on cancel | `QOLRules.asi`, `PayOnQueue=` | — | **built**, now the [`rules/`](../rules/README.md) layer |
| [QOL-8](#qol-8-long-moves-on-the-map-go-to-warp) | Long moves on the map go to warp | `QOL.asi`, `WarpDistance=` | A stock peer in a network game | **done** (`QOL.asi`), bench |
| [QOL-9](#qol-9-the-view-reaches-across-the-map) | The view reaches across the map | `Render3D.asi`, `[View] Distance=` | — | **done**, render3d's since qol 2.2.0 |

## Settings live

`QOL.asi` describes its keys to the Refit screen (`settings.toml`, turned into
`settings_table.h` by `tools/settings/gen.py`; contract: `tools/settings/README.md`) and read them through those
rows, `RefitPlayer.ini` in front of the plugin's ini. The code never holds a default of its
own: it was a `PanSpeed` of 1 in the code against 2.5 shipped, and the table's 2.5 won.

**The patches stay installed and are gated.** A key at 0 used to leave its patch out; now
the patch goes in whenever its site bytes match and the patched code reads a flag, so
`QOL_SettingsChanged` (called by UI.asi's hub on the game thread after the overlay
changes) can flip it. The log keeps the lines it always had and adds `gate:` lines for a
patch that is in but switched off.

| Key | Apply | How it flips |
|---|---|---|
| `PanSpeed` | live | the stub keeps the value `RTS_CFG.h` gave (`g_panOrig`) next to the factor; a change stores `orig x factor` again. 1 is a factor of 1: the store is unchanged |
| `ShiftAddsToGroup` | live | `group_stub` tests the flag; the one byte that moves Shift's old meaning to Alt is written back or forth |
| `StationGroups` | live | every station hook tests the flag (`sel_pre`, `bind_pre`, `producer_group`, `one_kind`, `focus_stub`) and takes the stock path when it is 0. Groups that already hold stations keep them |
| `WarpDistance` | live | `warp_pre` already read the distance; 0 does what stock's Alt block did |
| `MaxSelection` | restart | the selection array is moved at start; a bigger or smaller one is never re-applied |

`PayOnQueue`, the one row the players must agree on, is `QOLRules.asi`'s, in
[`rules/`](../rules/README.md).

## Hooks: what the plugins read of Armada2.exe

Every address, field offset and signature of `QOL.asi` is in the symbol profile
`profiles/<build>/qol.toml` (`profiles/gog-1.1/` for GOG's 1.1), generated into
`engine_syms.h` by `tools/engine/gen.py`
(`crates/a2engine/README.md`). A feature is a **hook**: its entries are checked in memory
by `a2e_ready` before the plugin writes anything, and a hook that fails stands down on its
own, with the reason in the log (`engine: <hook> stands down: ...`), on the feature's
`--- QOL` line (`NOT PATCHED: ...`) and in `QOL_SettingsNote`
for the settings screen. All hooks are asked first, before the first patch, because
patches change bytes another hook's check reads. Each asm stub continues through a
resolved variable (`pushl _a2s_qol_<id>; ret`), and reads a global as a load of the
variable and then of the global; field offsets are spliced in with `A2_STR(QOL_F_...)`.

| Hook | Setting | Entries | Without it |
|---|---|---|---|
| `qol.pan_speed` | `PanSpeed` | the parse's 25-byte pattern (`patch`) and the coefficient (`global`) | right-drag as stock |
| `qol.shift_group` | `ShiftAddsToGroup` | the Control test and the centre test in `mCheckGroupSelect` (`patch`), `mBindGroup` (`fn`), three continue points (`label`), the three modifier globals | number keys as stock |
| `qol.selection` | `MaxSelection` | `cOverViewImp` (`global`), its 31 selection-array sites and 3 caps (`patch`, each with the bytes it replaces) | selection of 16 |
| `qol.special_select` | `MaxSelection` | the five special-weapon array sequences (`patch`) and three continue points | selection of 16 (it is moved only with this hook) |
| `qol.stations` | `StationGroups` | the eight stub sites (`patch`/`fn`), nine continue points, the engine functions and RTTI the stations code calls, and the object fields | stations as stock |
| `qol.bar_keys` | `BarKeys` | the call of `mProcessKeyboardInput` in `PopupPaletteImp::Update` (`call`) and that function, the three hotkey counts and the six menu toggles, a button's state and press slot, and the click's sound (`operator new`, the `AudioSound2D` constructor, the click-sound global), `TextInput_IsActive`, `CinematicView::IsPlayingVideo` | position keys off, stock's bar keys |
| `qol.warp` | `WarpDistance` | the Alt block (`patch`) and its continue point, the warp globals and functions | moves as stock |

`MaxSelection` needs both of its hooks; the log names the one that failed.

## Things that apply to all of them

- **Multiplayer.** Anything that changes *what commands are issued* must keep issuing
  the stock commands, so that a refit player can play against a stock player and every
  peer simulates the same game. QOL-4, 5 and 6 should be pure client-side decisions,
  resolved into ordinary commands before they leave the machine. QOL-3 might not be.
  **QOL-7 is not**: it changes when the bank is charged, and every peer simulates the
  bank, so every player needs it.
  If a bigger group changes the size of a command the game sends, it changes the
  protocol, and both sides would need the plugin. Test each one with
  `./a2test run multiplayer-two-players` and the `multiplayer-online-*` scenarios.
- **Saves.** Control groups are saved with the game. Anything that changes what a group
  can hold (QOL-3, QOL-5) has to load and save correctly. It also has to fail safe in
  two cases: a refit save loaded with `./a2mod stock`, and a stock save loaded in refit.
- **The AI.** It builds and selects through the same game objects. QOL-6 must apply to
  the player's input only, never to the AI's own build logic. QOL-7 is a rule of the
  game, so the AI plays by it too (see QOL-7).
- **No game files in the repository** (hard rule 8). `Input.map` and `RTS_CFG.h` are
  game files. A change to them ships as a script that edits the player's own copy, with
  a backup and `--revert`, the way the retired `ui-widescreen.py` edited `misc/gui_*.cfg`.
  It never ships as a modified copy of the file. The UI's Keys tab does this for
  `Input.map` in game (`ui/MENUS.md`, "The Keys tab").
- **Which plugin.** See "Two plugins" above; each idea below says which.

---

## QOL-1: Grid hotkeys for the button bar

*Done. First as `GridLayout.asi` (retired), then drawn by the UI
(`UI.asi`, [`ui/HUD.md`](../ui/HUD.md)); since phase 9 the keys are this plugin's
(`BarKeys=` in `QOL.ini`, `bar.c`) and the UI keeps the layout and the drawing.
Stock-compatible: a key sends the ordinary command its button sends. What follows
the next section is the plan as it was written.*

### What `bar.c` does, and what it reads

The stock bar (`PopupPaletteImp`) keeps running: its `Update` works out which buttons the
selection and the open menu show, and a press goes through the button's own press slot
(vtable `+0x20`), which hands its command or mode to the bar. So a key is "press the
button of that cell", and which button sits in which cell is the UI's layout.

- **The wrap.** The call of `mProcessKeyboardInput` in `Update` (made when something of
  the player's is selected and the game is not paused) is re-pointed at a function of
  ours that calls the stock one, then `UI_BarStep` (the UI presses its queued clicks and
  takes the layout of this update's buttons), then reads the keys. The wrap checks the
  call's target first (`a2e_ready`).
- **Stock's keys are global per selection**, not per open menu (F5 builds from the top
  level whatever menu is open). With the grid's keys on, the three hotkey tables'
  counts (commands `+0xcc`, build items `+0xd8`, special weapons `+0xe4`) are set to
  zero for the stock call and put back after, and the six menu toggles' control
  pointers (a global array that `Init` fills: popup, build, orders, trade, AI,
  formations) point at a zero for good. Nothing outside the bar reads them. The bar's
  vtable is never patched.
- **The keys** are scancodes (Q W E R T / A S D F G / Z X C V B by default), turned into
  virtual keys with `MapVirtualKeyA` and read with `GetAsyncKeyState`, so they are
  positions on any layout. A key goes down once. Which key presses which cell is
  `[Keys] Cell1`..`Cell15` in `QOL.ini` (row by row, `Cell1` top left), each a key by its
  name in the game's `Input.map` (`a2fmt::inputmap::KEYS`; `bar.c` holds the subset that is
  not extended, with each one's scancode and its label on the button), read at start and
  again by `QOL_SettingsChanged`. The Refit screen's Keys tab rebinds them
  (`[QOL.Keys]` in `RefitPlayer.ini`, `ui/MENUS.md`, "The Keys tab"); a name the grid
  cannot use keeps the cell's default (`bar: [Keys] Cell3=... is not a key the grid can
  use`), and a change is logged (`bar: Cell3 E -> F1`). Nothing happens while chat has the keyboard
  (`TextInput_IsActive`), Ctrl or Alt is held, or the game is not the foreground window.
- **A press** is what releasing a click does: nothing for a disabled button, else the
  click (a fire-and-forget `AudioSound2D` made with the game's `operator new`, named by
  `StandardButton::s_buttonClickSound`, unless a video plays) and the press slot.
- **The two plugins meet at three exports**, each found by name in the loaded modules
  (`a2_find_export`; UI.asi loads after this plugin, so the lookup repeats until it
  succeeds): the UI's `UI_BarCell(cell)` (the stock button in cell 0..14, or NULL),
  the UI's `UI_BarStep(palette)` and this plugin's `QOL_BarLabel(cell)` (the key's
  label, NULL while the keys are off) and `QOL_BarKeyName(cell)` (the key's `Input.map`
  name whether the keys are on or off, for the Keys tab). **The keys work only while `UI_BarCell`
  exists and gives a cell**, i.e. while the UI's bar is drawn; otherwise
  they stand down with `bar: no refit bar, position keys off` in `QOL.log` and the stock
  bar keys run.

**Problem.** The button bar's hotkeys are hard to use without looking. Each building's
build menu binds its items to `F1`–`F12` by slot. The menus themselves sit on letters
spread over the keyboard (`B` build, `C` orders, `V` trade, `N` AI, `F` formations,
`X` palette, `R` recrew, and more).

**Wanted.** A layout like StarCraft II's grid: the item at a given position always sits
under the same key, and the keys mirror the layout (`Q W E R T` / `A S D F G` /
`Z X C V B`). The hand never leaves the left side of the keyboard.

**Today.** Every binding is in the game's `Input.map`: plain text, one block per action,
492 of them, with `+` and `-` modifier lines. The build entries are named per unit
(`bc_fscout`, `bc_fdestroy2`, …) under per-building headings. So a different mapping
may need no code at all.

**Approach, in order of cost.**

1. **Key remap only.** A script rewrites `Input.map` so that build, research and order
   slots map to grid keys by position. Whether this works depends on one question: are
   bindings scoped to the open menu, or global? If they are global, `C` cannot be both
   "orders menu" and "slot 13". In that case the grid needs a prefix: `B` opens the
   build menu, and the grid keys act while it is open. Armada already works this way
   with `B` then the F-keys.
2. **Grid layout of the palette.** The floating button palette (`X`, `popupPalette*`
   in `misc/gui_*.cfg`) could be laid out as a 5×3 grid instead of a bar, with each
   button labelled with its key. This is HUD work (`ui/HUD.md`), and the palette's width keys
   are special-cased there (`popupPaletteXA`/`XB`, "The references the stock HUD is drawn against").
3. **Labels.** Show the grid key on each button, not only in the tooltip.

**Open questions.** Are `Input.map` bindings global or scoped to the open menu? How many
slots does the busiest menu have? (The Construction Ship's build list is the longest
block in `Input.map`.) Can one action carry two bindings, so the stock keys keep
working beside the grid?

**Done when.** The bench can build every unit of one faction with only grid keys. Then,
in a real game, the user doesn't have to look down.

---

## QOL-2: Configurable right-drag pan speed

*`QOL.asi`, stock-compatible. Built: `PanSpeed=` in `QOL.ini`, default 2.5.*

**Problem.** Right-click-drag panning is slow on a large map at high resolution.

**Wanted.** A *Pan speed* slider in the game's options. Until then, a setting.

**The values.** The scroll speeds are plain values in `RTS_CFG.h`, under
`CAMERA CONTROL`:

| Key | Stock | What it does |
|---|---|---|
| `FASTSCROLL_COEFFICIENT` | 0.005 | right-drag pan only; no acceleration ramp |
| `SCROLL_COEFFICIENT` | 90000 | scales *every* pan path |
| `MAX_SCROLL_SPEED` / `INITIAL_SCROLL_SPEED` | 2.0 / 1 | ceiling and floor of the held-scroll ramp |
| `SCROLL_BORDER_WIDTH` | 2 | the edge-scroll band, in pixels |

The edge and keyboard scroll speeds the options screen sets live in `ARMADA.PRF`, not
here; right-drag ignores both, so `FASTSCROLL_COEFFICIENT` is the one value for it.

**Why not edit the file.** Editing `FASTSCROLL_COEFFICIENT` works, and the removed
`gameplay/` layer did it (with the other scroll values, at 3x). But a network game
compares a CRC of `RTS_CFG.h`'s bytes between nodes:
`TransportNetwork::ProcessPacketCRC` (`0x560af0`) reports *"EXE / RTS_CFG.h files do
not match node %d"*, and the CRC it compares is `CrcFile` on `rts_cfg.h` (called at
`0x55e63b`, beside the one for the exe). So an edited file shuts the player out of every
game with someone whose file is stock — for a value that only moves this player's
camera, and so cannot desync anything.

**How `QOL.asi` does it.** The `RTS_CFG.h` parser looks each key up by name; for
`FASTSCROLL_COEFFICIENT`, if found, it stores the parsed value with one
`fstp dword [0x70fbb4]` (at `0x491777`). Those six bytes become `call pan_stub; nop`:
the stub multiplies the value by `PanSpeed` and does the store itself. The only reads of
`0x70fbb4` are the two multiplies in `cOverViewImp::mMouseRightDrag` (`0x5268d7`,
`0x5268f5`), the x and y of the pan, so nothing but right-drag changes. Each parse scales
its own fresh value, so a second parse cannot compound it; a file without the key keeps
the compiled default, unscaled. The site's bytes are checked first, so another
`Armada2.exe` leaves the plugin inert and says so in `QOL.log`.

`ASI` plugins load before the game reads `RTS_CFG.h`, so the patch is always in place
for the parse. Seen on the bench (2026-10-03, at `PanSpeed=2`): `QOL.log` reads
`RTS_CFG.h parsed: right-drag FASTSCROLL_COEFFICIENT now 0.0100 (file value x 2.00)`
with the clone's file still at the stock 0.005, and a 200 px right-drag in the first
Federation mission moved the view about 160 px.

`install.sh` runs `rts-cfg-check.py --fix` first: if the live file's
`FASTSCROLL_COEFFICIENT` differs from the stock `RTS_CFG.h.a2neb-backup` beside it (a hand
edit, or the old `gameplay/scrollspeed.py`), that one line goes back to stock, since the
plugin would otherwise scale the edit. Every other line is the player's and is left
alone, with a warning if the file still differs from stock.

**Still open.**

1. **The slider.** The options screen is a shell menu, a Win32 dialog that the UI's menu
   host already hosts (`ui/MENUS.md`). A new slider means a new control on that dialog,
   saved with the other options (`ARMADA.PRF`, which carries no network check), and read
   by `QOL.asi` in place of `PanSpeed=`. The fallback is to apply the mouse scroll
   speed slider's value to right-drag too.
2. **Resolution.** Does a right-drag pan's speed depend on the resolution? If so, the
   default should scale with the back buffer, as the HUD does.

**Done when.** At 3440×1440, one right-drag gesture crosses a useful part of the map,
and the setting survives a restart.

---

## QOL-3: Larger control groups

*`QOL.asi`, stock-compatible. Built: `MaxSelection=` (default 40, 17 to 120; 16 or less
leaves it stock).*

**Problem.** A control group (`Ctrl+number` to store, `number` to recall) holds only 16
units. A fleet has to be split across several groups.

**Wanted.** A much higher limit, or none.

**What limits it.** The *selection*, not the group. A stored group is a `list_array` of
entity ids with no limit of its own (`cOverViewImp::mBindGroup`, 0x520c40, replaces it
from the selection with no cap). But a group is built from the selection and recalled
into it, and the selection is 16: `cOverViewImp` (one static object at 0x768e40,
`g_pOverView` at 0x768e3c points to it) keeps a count at +0xb8 and an array of 16 entity
ids at +0xbc, with live fields from +0xfc on, and `cOverViewImp::Select` (0x51f600)
refuses a 17th object. Two more caps of 16: `mBindGroup`'s add path (`Ctrl+Shift+N`,
0x520e97) and `mEditModeSelect` (0x5232e3).

**How `QOL.asi` lifts it.** The array moves into the plugin. All 31 places that address
it are `cOverViewImp` methods that reach it as `this+0xbc` with a 32-bit displacement or
immediate, so each is rewritten in place to point at the plugin's array, without
changing an instruction's length; the count stays at +0xb8. The three `cmp ...,0x10`
become `MaxSelection` (an 8-bit immediate, hence 120 at most). Every site is checked
byte for byte before any is written, so a different exe leaves the selection where it
was, in one piece.

**Who else reads the selection.** Everything outside `cOverViewImp` goes through its
`GetSelectNum` / `GetSelectList` / `GetSelectedEntityIds` (vtable +0x94/+0x98/+0x9c),
about 40 call sites, and copies into growable arrays: the button bar
(`PopupPaletteImp`), `ActionMode::GetAction` (its two stack arrays are vote counters per
action type, not per ship), the radar, `CommDisplay`'s give-units button. The selection
panel (`ShipDisplay`) has 16 icon slots and fills them from the first 16 ids; its
loops that would run to the selection count (`SimulateMultiObject`,
`DisplayMultiObject`) are never called. So a bigger selection shows its first 16 icons.

**Except the special weapons, which crashed 1.1.0.** For each special weapon of the
selection, `PopupPaletteImp::mSetupSpecialWeapons` (every frame, to set the button) and
`mQueueSpecialWeaponCommand` (to fire it) collect the selected ships that can use it
into a 16-entry array on their own stack, unbounded: `ebp-0x78` with a container right
after it at `ebp-0x38`, and `ebp-0x80` below that function's other locals. Seventeen
Galaxy or Vor'cha class ships overran the first: the frame loop never finished (a black
screen, the game at full CPU) or the heap was damaged and a ship's `CraftProcess::Attack`
later called through a destroyed weapon system (`R6025`, pure virtual call). Ships
without a special weapon never enter the array, which is why 40 Defiants and Intrepids
were fine. `QOL.asi` moves both arrays to buffers of 120; they are addressed in five
places (0x4fd32e, 0x4fd3af, 0x4fd40d, 0x4fd8f1, 0x4fd95d), each rewritten in place or
as a jump to a stub, and all five are checked before the selection is touched: if they
do not match, the cap stays at 16. The audit above missed them because it looked for
the selection being copied; these copy a per-weapon subset of it.

A bigger selection is also sent somewhere the audit above did not list: `GameObject::Select`
reports each selected handle to the transport (`MySelectedAdd`), which keeps a
`std::set` of them and, in a network game, sends it to allied co-players
(`TransportNetwork::MySelectedTransmit`). It has no fixed size.

**The network.** An order goes out as `NetOrderObjects`: two bytes, a 32-bit count, then
that many handles. The receiving end hands count and handles to
`GameObject::DeQueueCommand`, which walks however many arrive. A stock player therefore
carries out an order for 40 ships like any other, and nothing about the groups
themselves is sent. This is why QOL-3 is in `QOL.asi` and not the rules plugin; a
network game against a stock player is the check still to make.

**Seen on the bench (2026-10-04).** 30 scouts: `Ctrl+A` selected 30, `Ctrl+1` stored 30,
`1` recalled 30 (each read from the game's memory, not the panel), and a move order took
all 30 across the map. The panel shows 16 of them.

**Seen on the bench (2026-10-05).** 30 Galaxy class ships against Borg cubes: `Ctrl+A`
selected 30 with the frame loop running, and the special weapon (saucer separation) for
all 30 filled 30 entries of the moved array; then 40 selected of the 60 separated ships.
The Executioner save that crashed 1.1.0 with 18 Vor'chas selected was the original
report.

**Open.** A count, or pages, on the selection panel for more than 16. The frame rate
with 40 ships selected (seen in game with Defiants and Intrepids, gone on deselecting)
is not measured yet. Not checked yet: saving and loading a game with a group over 16,
and a two-player game against stock.

---

## QOL-4: Shift+number adds to a group

*`QOL.asi`, stock-compatible. Built: `ShiftAddsToGroup=` (default 1).*

**Wanted.** As in StarCraft II, `Shift+number` adds the current selection to that group
and keeps what is already in it. (`Ctrl+number` keeps its meaning: replace the group.)

**Stock.** `Input.map` binds `group_select_N` to the bare number key, with any
modifier; `cOverViewImp::mCheckGroupSelect` (0x521110) reads the modifiers itself from
`g_pCommandControl` (0x76133c) and `g_pCommandShift` (0x761340):

| Keys | Stock |
|---|---|
| `N` | select group N; a second `N` within 750 ms centres the camera on it |
| `Shift+N` | select group N and centre the camera at once |
| `Ctrl+N` | `mBindGroup(N, true)`: the group becomes the selection |
| `Ctrl+Shift+N` | `mBindGroup(N, false)`: the selection is added, the group kept |

A unit is in one group at a time: binding takes it out of every other group first. A
building goes into a second set of ten groups (`+0x1c4`, chosen by a flag at +0x20d of
the first selected object's class), which recall falls back to when the first set's
group is empty.

**With `QOL.asi`.** At 0x52129a, where the function tests Control to choose between
binding and selecting, a jump to the plugin sends `Shift+N` without Control to
`mBindGroup(N, false)`, the call stock makes for `Ctrl+Shift+N`; every other combination
goes back to stock. `Shift+N`'s old meaning moves to `Alt+N`: the one read of
`g_pCommandShift` that decides "centre now" (0x521350) reads `g_pCommandAlt` (0x761344)
instead. Nothing in `Input.map` uses `Alt` with a number key. With nothing selected,
`Shift+N` does nothing and the group is kept.

**Seen on the bench (2026-10-04).** Select A, `Ctrl+1`; select B, `Shift+1`; deselect;
`1` selects A and B. `Alt+1` after panning away selects both and centres the camera;
`1` selects without moving it. `Ctrl+1` on B alone replaces the group.

---

## QOL-5: Buildings in control groups

*`QOL.asi`, stock-compatible. Built: `StationGroups=` (default 1).*

**Wanted.** `Ctrl+number` stores stations (shipyards, research stations) like ships,
`number` selects them again, so their build menu is one key away, and a second
`number` centres the camera on them, as it does for ships. One kind of station to a
group is enough (decided 2026-10-05).

**Stock (seen in game, 2026-10-05).** One station can be stored, and `Shift+number`
adds a second; but recalling the group selects only the one added last. Binding a ship
to a group that held a station leaves the station's group number drawn beside it. A
second press of the number does not move the camera to a station group.

**Why.** `cOverViewImp` keeps two sets of ten groups (`cGroup`, 12 bytes: the list, and
the group number at +8 that is copied to the object's label, `GameObject` +0x120):
ships at +0x14c and stations at +0x1c4. Which set an object goes to is its class's:
`mBindGroup` (0x520c40) casts the object's `GameObjectClass` (+0x40) to `CraftClass`,
whose byte at +0x20d marks a station. Recall (`mCheckGroupSelect`, 0x521110) takes
the ships' group N, or the stations' when that is empty, and selects each member
through `cOverViewImp::Select`. Three things then go wrong:

- **`Select` (0x51f600) never holds two stations.** With one object selected that is
  not this player's ship, it deselects that object before taking the new one; with
  anything selected, it refuses anything but this player's ships. Recall therefore ends
  on the last station it selected.
- **`mBindGroup`'s replace (`Ctrl+N`) empties only the ships' group N**, and only when
  the selection is ships: stations always take the add path, so `Ctrl+N` on a station
  adds to the group, and a ship bound over a station group leaves the station in the
  stations' group N with its label (`cGroup::RemoveShip` and `RemoveThisShip` are what
  reset a label to -1).
- **The double tap (`mFocusCameraOnShipGroup`, 0x520fb0) reads the ships' group only.**
  It averages the living members' positions and points the camera at the member
  nearest that mean (camera manager's vtable +0x58), the "centre of the bulk".

**With `QOL.asi`.**

- `Select`, where it tests "one is selected" (0x51f67b): when the selection is this
  player's stations, another station of the same `GameObjectClass` is taken as stock
  takes a second ship. Anything else selected over several stations deselects them
  all first, as stock does over one. Mixing ships and stations stays impossible. This
  is the one gate, so a click with Shift and the double click (everything of that
  type on screen) select several stations too: double-clicking a yard selects every
  such yard in view.
- `mBindGroup`, at its entry: `Ctrl+N` empties both sets' group N first, labels
  included, so the group becomes exactly the selection. `Shift+N` (and `Ctrl+Shift+N`)
  refuses ships into a group that holds stations, stations into one that holds ships,
  and a second class of station; a group whose members are all gone counts as empty.
- `mFocusCameraOnShipGroup`, where it picks the group (0x520fcf): the stations' group
  N when the ships' is empty, the choice recall makes. The double tap and `Alt+N`
  (QOL-4) centre on the station nearest the group's middle.
- `Producer::FinishBuild` (0x4b8f50), where it hands the unit it has built to
  `AddCraftToGroup` (`cOverViewImp`'s vtable +0xd4) with the producer's group number
  (0x4b9127): a station's builds get -1, no group. Stock gives a yard's ships the
  yard's group, so they landed in the ships' group N, which recall prefers, and the
  key selected the new ships instead of the yards (seen in game, 2026-10-05). A unit
  that changes into another (`Evolver`, `s_GiveSameStatsAndGroupAndSelection`) keeps
  its own group as in stock.

Groups are this player's alone and saved as stock saves them (a station group could
already hold several), so nothing about this reaches other players or the save.

**Seen on the bench (2026-10-05)**, `testbench/scenarios/qol-station-groups.md`, on the
`stations` scene (three yards, an advanced yard, a research station, two ships):
three yards select together; an advanced yard or a ship replaces them; a scout built at
a yard of group 2 joins no group, and `2` still selects the yards; `Ctrl+1` on
three yards, then `1` from a ship, selects all three labelled 1; `Ctrl+1` on a ship
empties the yards' group and clears their labels; `Shift+2` refuses a ship and the
advanced yard and takes a third yard; `2 2` and `Alt+2` move the camera to the middle
yard, and a ship group's double tap still centres on the ship.

---

## QOL-6: Production spread across a group of buildings

*`QOL.asi`, stock-compatible: one ordinary order to the chosen building. Built for
stations, with QOL-5 (`StationGroups=`).*

**Wanted.** Several buildings of the same kind in one group, for example three
Federation shipyards on `4`. A build order given to the group goes to *one* of them,
chosen by:

1. the building with the fewest items in its queue (counting the one in progress);
2. on a tie, round-robin. The group remembers which building got the last order and
   moves on to the next one.

`4`, then `B`, then the Akira key pressed five times should give 2 + 2 + 1, not 5 on
one yard and nothing on the others.

**Stock.** The button bar enables its build button for exactly one producer
(`PopupPaletteImp::mUpdateButtonsAndPosition`: the first object's class can build, and
the count is 1), so with QOL-5's several yards selected the build menu would be out of
reach. Past that gate, stock already handles several: `CheckCanExecute(ModeInfo)`
queues a build (`GameObject::QueueCommand`, command 0x19, the class to build) at
**every** selected producer, and `CheckCanExecute(CommandInfoClass)` sends a cancel
(command 0x13) to every one as well. Its button is enabled when the *first* producer
has more queued than the one in progress (`Producer::BuildQueueSize` > 1).

**With `QOL.asi`**, for a selection of this player's stations of one class:

- the build button is enabled (0x4fc72c);
- a build order goes to one station: the shortest `BuildQueueSize`, and on a tie the
  next after the one that got the last order, in selection order (0x4fc486, in place
  of stock's loop);
- a cancel goes to the station with the longest queue (0x4fc0a6), and the longest
  queue is what enables its button (0x4fbf83). Stock's cancel takes the last item that
  is waiting, not the one in progress.

Each is the single-station order a stock player's game receives. One producer, or
anything that is not stations of one class, goes the stock way. The AI never uses the
button bar.

**Seen on the bench (2026-10-05).** Three yards, five orders for a slow ship: 2 + 2 + 1,
and the first three are charged as they start, the other two when they do (stock).
Five more from empty queues continued the turn: 2 + 1 + 2. A cancel with queues 2, 2, 1
took yard1 to 1, the next yard2 to 1; with yard1 at 1 (only the one in progress) and
yard3 at 3, the button stayed enabled and the cancel took yard3 to 2.

**Open.** Construction ships: should a group of them place each structure with a
different ship? They are ships, so QOL-5 and this leave them stock. Weighing *time
remaining* rather than item count was not asked for; count is predictable.

---

## QOL-7: Pay when queuing, refund on cancel

Moved to the `rules/` layer (rules 1.0.0, qol 2.0.0): [`rules/README.md`](../rules/README.md#qol-7-pay-when-queuing-refund-on-cancel). `QOLRules.asi` is no longer part of this layer.

---

## QOL-8: Long moves on the map go to warp

*`QOL.asi`, stock-compatible: the minimap's own order. Built: `WarpDistance=` (default
1100; 0 leaves moves stock).*

**Problem.** A move ordered on the minimap goes to warp; the same move ordered on the
map does not, unless Alt is held, which nothing in the game tells the player. Zoomed out,
a ship sent from the middle of the screen to its edge crawls there at impulse.

**Wanted.** A move on the map past some distance goes to warp, as if it had been
ordered on the minimap.

**Stock.** A move is one of two commands: `GO` (4) or `GO_WARP` (0x2b), each with a
`_SINGLE` form for a lone ship (`AiCommandToName`'s table). Every frame,
`cOverViewImp::mNormalModeInput` (0x522c40) works out what a click on the map would do
(`ActionMode::GetAction`) and keeps it at `cOverViewImp+0x3c`, with the map point under
the cursor at +0xa4. The click handlers (`mNormalModeLeftClick`,
`mNormalModeRightClick`, `mBroadcastAction`) send whatever is there, and the cursor is
drawn from it too. At 0x522cb8 that function turns a `GO` into `GO_WARP` when Alt
(`g_pCommandAlt`, 0x761344) is held. The minimap's `RadarComponent::Simulate` does the
opposite: `GO_WARP` unless Alt is held (0x4e8bfa).

What a ship does with `GO_WARP` is decided where it is carried out, on every machine
alike. `CraftProcess::Handle_New_Command` sets the move up as a `GO`, and then calls
`cCraftControl::TryWarp(true)` only if that ship is farther than
`cfgTOO_CLOSE_TO_WARP` (800 in `RTS_CFG.h`) from where it is going. `TryWarp` arms warp
only if the ship's warp speed is above its speed and warp is allowed (`g_allowWarp`, the
mission scripts' `AllowWarp`; `GameSetup::isAllowWarpSpeed`, the setup screen). On the
way, `cHoverCraftControlBase::mWarpCheck` drops warp near planets and black holes
(`mPreWarpCheck`) and when the destination is closer than the ship can stop from warp.
So a `GO_WARP` cannot make a ship warp where the rules say it may not, and a ship
close to its destination moves at impulse while the rest of the selection warps.

**With `QOL.asi`.** Stock's Alt block (0x522cb8 to 0x522cdf) becomes a jump to the
plugin. Alt held still turns a `GO` into `GO_WARP` as in stock. Without Alt, so does a
`GO` whose selection is on average farther than `WarpDistance` from the point under
the cursor: this player's selected ships that can warp, by the straight-line distance
from each one's position (+0xac) to the point, averaged. A ship can warp when its
control (`Craft+0x1b0`) has a warp speed (+0xc, `cCraftControl::SetWarpSpeed`) above
its speed (+0x8), `TryWarp`'s own test; a selection with none such, or a game with warp
turned off (`g_allowWarp`, `GameSetup::isAllowWarpSpeed`, as `TryWarp` reads them),
stays an ordinary move. Counting every ship turned the cursor blue for ships that
cannot warp (seen in game, 2026-10-07). One order goes to the whole selection,
so the choice is one per order. The average is what a scattered selection sent to its
own middle needs: it stays an ordinary move. The flag bit 0x4000 is kept as stock
keeps it. The order sent is the ordinary `GO_WARP`, the one a stock player's minimap
sends, so a stock player in the same game carries it out like any other.

Since the cursor is drawn from the same field, it shows the choice before the click:
stock's green move ring near, the game's blue warp ring past the distance.

**Why 1100.** It is `cfg_MIN_MOVE_WARP_DISTANCE` in `RTS_CFG.h` ("At what range will
the AI try to warp?"), so the player warps where the computer players do. Measured on
the bench at 1920x1080 (2026-10-07): at the default zoom a pixel is about 1.15 units,
so the side of the screen is about 1100 from its middle; zoomed all the way out it is
about 1.75 units, about 1700. Ordering from the middle to the side therefore warps when
zoomed out, and is on the edge of it at the default zoom.

**Seen on the bench (2026-10-07)**, `testbench/scenarios/qol-warp.md` on the `warp`
scene (three destroyers): zoomed out, a right-click near the right edge (about 1600
away) showed the warp ring, and the ships went from 3000 to 4567 at up to about 700
units a second (sampled by hand, about a second apart). A right-click about 1000 away
showed the green ring and ran at about 240 a second. The scripted run showed the same
two rings; the time warp takes to spin up was not measured.

**Open.** A network game against a stock player (the order is the minimap's, so it
should simply be carried out). Whether the threshold should follow the zoom: at the
closest zooms, a move across the screen stays an ordinary one.

## QOL-9: The view reaches across the map

Moved to render3d in qol 2.2.0: the key is `[View] Distance` in `Render3D.ini` and the
findings are in [`render3d/README.md`](../render3d/README.md#view-distance). `QOL.ini`'s
old `ViewDistance=` is carried over by `tools/settings/move-keys.py` and the release
installers.
