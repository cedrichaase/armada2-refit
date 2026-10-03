# Ideas: gameplay quality of life

Changes to how Armada II *plays*, as opposed to how it looks. None of these is built
yet. Each entry says what is wrong today, what we want, what is already known, and what
still has to be measured. When an idea gets built, its layer's README takes over the
*why* and this file just links to it.

Status values: **idea** (not investigated), **scoped** (approach known, nothing built),
**in progress**, **done** (link to the layer).

| ID | Idea | Surface | First step | Status |
|---|---|---|---|---|
| [QOL-1](#qol-1-grid-hotkeys-for-the-button-bar) | Grid hotkeys for the button bar | `Input.map`, then maybe HUD layout | Remap the build slots, try on the bench | idea |
| [QOL-2](#qol-2-configurable-right-drag-pan-speed) | Configurable right-drag pan speed | `RTS_CFG.h`, then an options slider | Try the existing `RTS_CFG.h` experiment | scoped |
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
- **Where it lives.** These belong together in one new layer, say `controls/`, with one
  plugin (`Controls.asi`) and one `.ini`, switched by `a2mod` like the others. QOL-2's
  first step may be just a config edit. The layer gets its README and CHANGELOG when the
  first idea is built.

---

## QOL-1: Grid hotkeys for the button bar

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

**Problem.** Right-click-drag panning is slow on a large map at high resolution.

**Wanted.** A *Pan speed* slider in the game's options. If that is too hard, the
existing mouse scroll speed slider should drive right-drag panning too.

**Today.** The speeds are plain values in `RTS_CFG.h`, under `CAMERA CONTROL`:

| Key | Stock | What it does |
|---|---|---|
| `FASTSCROLL_COEFFICIENT` | 0.005 | right-drag pan only; no acceleration ramp |
| `SCROLL_COEFFICIENT` | 90000 | scales *every* pan path |
| `MAX_SCROLL_SPEED` / `INITIAL_SCROLL_SPEED` | 2.0 / 1 | ceiling and floor of the held-scroll ramp |
| `SCROLL_BORDER_WIDTH` | 2 | the edge-scroll band, in pixels |

There is already an experiment in the install, `RTS_CFG.h.scrollspeed`, with right-drag
at 3× (0.015). It is not the live file. Nothing records whether it was played.

**Approach.**

1. **Config edit.** Scale `FASTSCROLL_COEFFICIENT` alone, since it is the right-drag
   path and touches nothing else. Ship it as a script with a backup and `--revert`.
   `a2mod` treats `RTS_CFG.h` as the player's own file, so the script must change only
   this one key.
2. **Read at run time.** A `PanSpeed=` key in the layer's `.ini`, applied by the plugin
   where the game reads the coefficient. This is the step before a slider, and needs no
   file edit.
3. **Slider.** The options screen is a shell menu: a Win32 dialog, which `Menus.asi`
   already hooks (`menus/README.md`). A new slider means a new control on that dialog,
   saved with the other options (`ARMADA.PRF`), and read by step 2. The fallback is to
   take the mouse scroll speed slider's value and apply it to the right-drag
   coefficient.

**Open questions.** Is the game's scroll-speed slider stored in `ARMADA.PRF`, and what
does it scale today? Does a right-drag pan's speed depend on resolution? If so, the
default should scale with the back buffer, as the HUD does.

**Done when.** At 3440×1440, one right-drag gesture crosses a useful part of the map,
and the setting survives a restart.

---

## QOL-3: Larger control groups

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
  it is still waiting or already in progress. Whatever partial refund stock gives for
  an item in progress is replaced, not added to, so nothing is refunded twice.
- **Refund on destruction.** When a building is destroyed, everything in its queue is
  refunded in full, the item in progress included. The same applies when a
  construction ship with queued structures is destroyed. A building that is
  *captured* or *assimilated* should refund its old owner too. That is not decided
  yet, but it is the consistent reading.
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

**Open questions.**

- What does stock refund today when an in-progress item is cancelled? Measure it, so
  the 100% rule replaces it exactly.
- Captured or assimilated buildings: refund the old owner, and the new owner inherits
  an empty queue?

**Done when.** With 500 dilithium and an item costing 300, the first order drops the
bank to 200 and the second is refused. Cancelling the first restores 500. The
voice-over never plays during a normal build-up on the bench. A two-player bench game
with nine orders and three cancels ends with both peers showing the same bank.
Destroying a yard with three paid items returns all three costs. An order with too few
officers is refused. A save made mid-queue loads with the same bank and queues, and
nothing is charged a second time.
