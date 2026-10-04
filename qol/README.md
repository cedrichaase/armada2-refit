# qol — gameplay quality of life

Changes to how Armada II *plays*, as opposed to how it looks: the controls, the camera,
control groups, production. One of them is built (QOL-2, the right-drag pan speed, in
`QOL.asi`); the rest are planned here, each with what is wrong today, what we want, what
is already known and what still has to be measured. What is installed, at which
version, and whether it has been seen in game is in [`CHANGELOG.md`](CHANGELOG.md).

    qol/install.sh                  build QOL.asi and install it with QOL.ini
    qol/install.sh --pan-speed 3    ... with PanSpeed=3
    qol/install.sh --remove         take it out (QOL.asi, QOL.ini, QOL.log)
    qol/rts-cfg-check.py [--fix]    is RTS_CFG.h stock where QOL.asi needs it to be?

`./install` runs `qol/install.sh`; `./a2mod stock` / `refit` switches `QOL.asi` and
`QOL.ini` like the other plugins.

## Two plugins: what other players need too

A change either stays on this player's machine or it changes the game every machine in a
network game simulates, and the two cannot ship in one plugin:

- **`QOL.asi` — stock-compatible.** The camera, the keys, which ordinary command a key
  press turns into. A player with it can join a player without it, and nothing they
  simulate differs. Installed by default. QOL-2, 4, 5 and 6 belong here. QOL-1 became
  a plugin of its own, `GridLayout.asi` (`grid/`), so it can be switched separately.
- **A rules plugin (`QOLRules.asi`, not built yet) — every player needs it.** Changes to
  what the game *does* with a command: when the bank is charged, what a group can
  hold if that travels over the network. Every node must run it with the same settings,
  so it needs a check when a network game is set up (the `online/` layer is the place)
  and it must refuse or stand down rather than desync. QOL-7 belongs here, and QOL-3 if
  bigger groups change what is sent.

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
| [QOL-1](#qol-1-grid-hotkeys-for-the-button-bar) | Grid hotkeys for the button bar | `GridLayout.asi` (`grid/`) | — | **done** ([`grid/`](../grid/README.md)) |
| [QOL-2](#qol-2-configurable-right-drag-pan-speed) | Configurable right-drag pan speed | `QOL.asi`, `PanSpeed=` | A slider in the options screen | **done** (`QOL.asi`); slider open |
| [QOL-3](#qol-3-larger-control-groups) | Larger control groups | code | Measure the cap | idea |
| [QOL-4](#qol-4-shiftnumber-adds-to-a-group) | Shift+number adds to a group | code, `Input.map` | Check what Shift+number does today | idea |
| [QOL-5](#qol-5-buildings-in-control-groups) | Buildings in control groups | code | Check what Ctrl+number does on a building | idea |
| [QOL-6](#qol-6-production-spread-across-a-group-of-buildings) | Production spread across a group of buildings | code | Check what a build order does with several yards selected | idea |
| [QOL-7](#qol-7-pay-when-queuing-refund-on-cancel) | Pay when queuing, refund on cancel | code, every peer | Measure stock's charge and refund rules | scoped |

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
  a backup and `--revert`, the way `hud/ui-widescreen.py` edits `misc/gui_*.cfg`.
  It never ships as a modified copy of the file.
- **Which plugin.** See "Two plugins" above; each idea below says which.

---

## QOL-1: Grid hotkeys for the button bar

*Done, as a plugin of its own: [`grid/`](../grid/README.md), `GridLayout.asi`, so it can be
switched on and off by itself. Stock-compatible. What follows is the plan as it was
written; the README there has what was built and why.*

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
   button labelled with its key. This is HUD work (`hud/`), and the palette's width keys
   are already special-cased there (`popupPaletteXA`/`XB`).
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

1. **The slider.** The options screen is a shell menu, a Win32 dialog that `Menus.asi`
   already hooks (`menus/README.md`). A new slider means a new control on that dialog,
   saved with the other options (`ARMADA.PRF`, which carries no network check), and read
   by `QOL.asi` in place of `PanSpeed=`. The fallback is to apply the mouse scroll
   speed slider's value to right-drag too.
2. **Resolution.** Does a right-drag pan's speed depend on the resolution? If so, the
   default should scale with the back buffer, as the HUD does.

**Done when.** At 3440×1440, one right-drag gesture crosses a useful part of the map,
and the setting survives a restart.

---

## QOL-3: Larger control groups

*Rules plugin if bigger groups change what is sent; `QOL.asi` if not. Measure first.*

**Problem.** A control group (`Ctrl+number` to store, `number` to recall) holds only a
few units. A fleet has to be split across several groups.

**Wanted.** A much higher limit, or none.

**Today.** The cap's value and where it applies are not yet measured. Two limits may be
involved, and they must be told apart: the size of a *selection*, and the size of a
*stored group*. The selection panel in the HUD also shows a fixed number of unit icons.
A larger group must still display sensibly there, for example as a count or pages,
even if it shows only the first N icons.

**Approach.** Measure first on the bench: select 10, 20, 30, 50 ships and store each
selection as a group. Raising a fixed array size means a plugin that moves the group
storage to a larger buffer, everywhere it is read: recall, the HUD, saves and the
network. That is the expensive part.

**Open questions.** What is the cap, and is it per selection, per group, or both? Does a
group or selection command go over the network with a fixed number of units? (See
"Multiplayer" above.)

**Done when.** A group of 50 ships stores, recalls, saves, loads, and moves as one in a
two-player bench game.

---

## QOL-4: Shift+number adds to a group

*`QOL.asi`, stock-compatible.*

**Wanted.** As in StarCraft II, `Shift+number` adds the current selection to that group
and keeps what is already in it. (`Ctrl+number` keeps its meaning: replace the group.)

**Today.** `Input.map` binds `group_select_N` to the bare number key. Groups are
created by holding `Ctrl`, which the engine handles. Nothing yet says what `Shift+N`
does in stock.

**Approach.** Bind `Shift+N` to a new action in the plugin: read group N, append the
units not already in it up to the cap (QOL-3), and store it again. Do nothing on an
empty selection.

**Open questions.** Does `Shift+N` already mean something, such as adding the group to
the current selection? If it does, that meaning moves to another key. Ask before taking
it.

**Done when.** Select A, `Ctrl+1`, select B, `Shift+1`, `1` recalls A and B.

---

## QOL-5: Buildings in control groups

*`QOL.asi`, stock-compatible, if a group stays local; measure.*

**Wanted.** `Ctrl+number` stores a building (a shipyard, a research station) like a
ship, and `number` selects it again, so its build menu is one key away.

**Today.** Not checked: does the game refuse to group buildings, or drop them on
recall? Stations do have a selection and a build menu, so the HUD side exists.

**Approach.** Allow buildings into a stored group. Decide what happens with a mixed
group (ships and buildings): recall selects all of them, but a move order should only
reach the ships. Recall should not move the camera to a building unless the key is
double-tapped, if the game does that for ships.

**Done when.** `Ctrl+4` on a shipyard, pan away, `4`, `B`: the build menu of that
shipyard is open.

---

## QOL-6: Production spread across a group of buildings

*`QOL.asi`, stock-compatible: one ordinary order to the chosen building.*

**Wanted.** Several buildings of the same kind in one group, for example three
Federation shipyards on `4`. A build order given to the group goes to *one* of them,
chosen by:

1. the building with the fewest items in its queue (counting the one in progress);
2. on a tie, round-robin. The group remembers which building got the last order and
   moves on to the next one.

`4`, then `B`, then the Akira key pressed five times should give 2 + 2 + 1, not 5 on
one yard and nothing on the others.

**Today.** Not checked: what does a build order do with several producers selected?
It may go to all of them, or to the first one, or not be offered at all.

**Approach.** When a build order is issued with more than one producer selected, the
plugin picks one producer by the rule above and issues an ordinary single-building
order to it (see "Multiplayer"). Only producers that can build that item take part,
so a group with a shipyard and an advanced shipyard still routes each unit to a yard
that has it. A cancel goes to the yard with the longest queue of that item.

**Open questions.** Should the rule weigh *time remaining* rather than item count? A
yard with one long build may be busier than one with two short builds. Item count is
what was asked for, and it is predictable. Should a Construction Ship group work the
same way, with each structure placed by a different ship?

**Done when.** On the bench, three yards on one group, nine identical orders: each yard
gets three, and the units come out in the expected order.

---

## QOL-7: Pay when queuing, refund on cancel

*Rules plugin: every player needs it.*

**Problem.** Anything can be queued at any time. The cost is checked only when the queue
reaches that item. The check then fails, and the advisor's voice-over announces it,
often several times in a row. Construction ships do the same with queued structures.

**Wanted (decided 2026-10-03).** Resources leave the bank **the moment an item is
queued**, not when it starts building. An order the bank can't cover is refused right
away, so the queue only ever holds what is already paid for. **Cancelling a queued
item refunds its cost in full.** This is rule B ("stock minus what is already queued")
taken all the way: there is nothing to reserve, because the bank already shows what's
left. Rule A, which checks only the current stock, was rejected because five queued
items can still stall at the third.

**What it takes.**

- **Charge on enqueue.** When a player's order adds an item to a queue (shipyards,
  research, construction ships), check the bank against that item's cost. If it falls
  short, drop the order. Otherwise subtract the cost and queue the item.
- **Don't charge twice.** The game charges an item when it *starts* building, and that
  charge has to stop for items we have already paid for. The same goes for the game's
  start-of-build cost check and its voice-over: an item that was paid on enqueue can't
  fail it.
- **Refund on cancel: always 100%.** Cancelling refunds the item's whole cost, whether
  it is still waiting or already in progress. Stock already refunds an in-progress
  item in full (per the user, 2026-10-03), so that path stays as it is. Only the
  waiting items, which stock never charged, need our refund. Check on the bench that
  stock's refund matches what we charged, including crew and officers, so nothing is
  refunded twice or missed.
- **Refund on destruction.** When a building is destroyed, everything in its queue is
  refunded in full, the item in progress included. The same applies when a
  construction ship with queued structures is destroyed.
- **Refund on capture or assimilation.** The old owner gets its whole queue back,
  exactly as if the building had been destroyed. The new owner takes the building over
  with an empty queue (decided 2026-10-03).
- **Officers and crew are part of the check.** An order is refused if the player is
  short of officers or crew, counted the same way as the resources: what's free, minus
  what's already queued. Crew is a quantity in the bank, so it is charged on enqueue and
  refunded like the rest. Officers are a cap, not a stock, so each queued item takes its
  share of the cap when it is queued and gives it back on cancel or destruction.
  (This is the reading of "checked before". Confirm it when it is built.)
- **Spreading orders (QOL-6) needs no extra rule.** Each order is charged when it is
  placed, whichever building it goes to.

**Feedback.** No voice-over. Use the button's own disabled state, a short click, or one
line in the chat box (`notice()`, as `online/` does). Never a window of our own.

**Multiplayer: this one is not client-side.** Every peer simulates the bank. If only one
side charges on enqueue, the banks drift apart and the game desyncs. So the charge,
the refund and the start-of-build exception must run identically on every machine,
from the order as it arrives. That means every player in a game needs the plugin,
and the game has to refuse to start, or fall back to stock rules, when one doesn't
have it. The `online/` layer is the natural place to agree on this when a game is set
up. Single player and skirmish against the AI don't have this problem.

**The AI.** It queues through the same objects. The simplest rule is that it plays by
the same rule (charged on enqueue). That is fair, and it is deterministic as long as
every peer runs the plugin. Exempting it would mean telling its orders apart from a
player's. Measure whether the AI's build planning breaks when the bank drops early.
It may hold back a queue it would otherwise build in time.

**Saves change (decided).** Every queued item carries an "already paid" mark, and the
save has to hold it. Without it, a load either charges again when the item starts, or
builds it for free. The new save layout must be designed so that:

- a refit save loaded under `./a2mod stock` is refused cleanly, or loads with the
  queues emptied and refunded. It must never load into a wrong bank;
- a stock save loaded in refit treats its queues as unpaid, under stock rules, and is
  charged as each item starts, as it would have been;
- the mark goes in a place the stock loader ignores or rejects predictably. Measure
  how the stock loader reacts to extra data before choosing.

This makes the layer's eventual changelog a MAJOR version on the day it ships, because
saves made with it need it to load.

**Done when.** With 500 dilithium and an item costing 300, the first order drops the
bank to 200 and the second is refused. Cancelling the first restores 500. The
voice-over never plays during a normal build-up on the bench. A two-player bench game
with nine orders and three cancels ends with both peers showing the same bank.
Destroying a yard with three paid items returns all three costs, and so does losing it
to capture or assimilation, which leaves the new owner an empty queue. An order with too few
officers is refused. A save made mid-queue loads with the same bank and queues, and
nothing is charged a second time.
