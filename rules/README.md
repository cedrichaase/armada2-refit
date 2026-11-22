# rules — the rules layer

`QOLRules.asi`: the one change in this project that alters what every machine of a network
game simulates, so it is a layer of its own (`scope = 'match'` in `layers.toml`) and not
part of `qol/`, whose changes all stay stock-compatible. It holds QOL-7, pay when queuing
and refund on cancel (`PayOnQueue=`). What is installed, at which version, and whether it
has been seen in game is in [`CHANGELOG.md`](CHANGELOG.md).

    rules/install.sh                build QOLRules.asi and install it with QOLRules.ini
    rules/install.sh --remove       take it out (QOLRules.asi, QOLRules.ini, QOLRules.log)

`./install` runs `rules/install.sh`; `./install --without rules` takes it out; `./a2mod
stock` / `refit` switches `QOLRules.asi` and `QOLRules.ini` like the other plugins.
`PayOnQueue` ships on, and the installers select the layer by default.

**Where the log is.** `QOLRules.log`, in `%LOCALAPPDATA%\armada2-refit\logs\` like
`QOL.log` (written by `a2_log_path()` in `csdk/a2asi.h`).

## Must-match (contract C7)

- The protocol is the one `online/peer.c` has always spoken (`R_RULES`): joiners report
  whether they run the plugin with `PayOnQueue` on, the host answers whether everyone
  does, and the plugin latches the answer at `Program::GameOpenPreLoad`
  (`QOLRules_Wanted`, `QOLRules_Network`, `QOLRules_Reset`). Only *Internet - Online* has
  it; every other transport stands down.
- **A player without this layer** (left out of the install, or set aside by `./a2mod
  stock`) reports no rules, and the host plays stock rules for everyone: the game is
  never desynchronised, it just is not pay-on-queue. A host without the layer plays stock
  rules whatever the joiners have.
- `online/` finds the plugin by its export (`a2_find_export("QOLRules_Wanted")` in
  `csdk/a2asi.h`: module enumeration), never by the file name.
- When the plugin's settings table (`QOLRules_Settings`) is absent, the Refit screen's
  Gameplay tab shows one note row, "Pay on queue is not installed: network games you
  join play stock rules". The `PayOnQueue` row stays locked in network games.

## Settings live

`settings.toml` is turned into `settings_table.h` by `tools/settings/gen.py` (contract:
`tools/settings/README.md`); the key is read through its row, `RefitPlayer.ini` in front
of `QOLRules.ini`.

| Key | Apply | How it flips |
|---|---|---|
| `PayOnQueue` (QOLRules) | mission | the hooks are in whatever the key says; `g_pay` takes the key at `Program::GameOpenPreLoad`, so a queue is given back by the rules it was paid under. `QOLRules_Wanted`, which Online asks when a network game is set up, follows the key at once |

`PayOnQueue` is the one row the players must agree on (scope must-match); it is on in the
shipped `QOLRules.ini`.

## Hooks: what the plugin reads of Armada2.exe

Every address, field offset and signature of `QOLRules.asi` is in the symbol profile
`profiles/<build>/qolrules.toml` (`profiles/gog-1.1/` for GOG's 1.1), generated into
`engine_syms.h` by `tools/engine/gen.py` (`crates/a2engine/README.md`). A feature is a
**hook**: its entries are checked in memory before the plugin writes anything, and a hook
that fails stands down on its own, with the reason in the log and in
`QOLRules_SettingsNote` for the settings screen.

| Hook | Setting | Entries | Without it |
|---|---|---|---|
| `qolrules.pay_on_queue` | `PayOnQueue` | nine entries (`fn`, the first bytes copied behind a jump back), the queue call in `Handle_New_Command` (`call`), the bank and cost functions, the two local transports' vtables | stock payment |
| `qolrules.advisor` | `PayOnQueue` | `StartBuild`'s own advisor calls (`label`, `call`s), `ComputerVoice::Say`, `Race::GetInsufficentOfficerEventId`, `Entity::GetUserTeam` | a refused order is not announced |

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

**Feedback.** At first: no voice-over. Since issue #1 (qol 1.6.2) a refused order says
so the way stock says a stalled build does, with the same advisor line, once per click
at most and only to the player refused (see "The advisor on a refused order" below).
Never a window of our own.

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

---

## QOL-7: built (qol 1.5.0, now rules 1.0.0)

Where it stands. `QOLRules.asi` (`rules/rules.c`; its header comment holds the derivation)
charges at the build order's `PushBuildQueueItem` call and marks the item paid in bit 30
of its id; `StartBuild` gives a paid head back and lets stock charge it again (net zero,
no advisor); Pop, CancelBuild, ActDelete, ClearBuildQueue and ClearTeam give paid items
back; Save/Load carry the marks in the producer's id counter. A destroyed producer is
refunded through ClearTeam, which `GameObject::Cleanup` always calls and which zeroes the
item in progress, so it is refunded once; DestroyShip is not hooked (it would refund again,
and a craft that cannot die passes through it on every fatal hit). The officers of the
item in progress are stock's to return there when `mRemoveConstructionEffect` does
(qol 1.6.1). Measured on the bench: stock's crew refund is clamped to a cap (`AddCrew`),
so crew is put back directly; a start's give-back needs the same.

Network games: `online/peer.c` agrees the rules (`R_RULES`: joiners report, the host
answers with whether everyone has the plugin; `QOLRules_Network`), and the plugin latches
the answer, until the next game opens (`Program::GameOpenPreLoad`). Only *Internet –
Online* has it; other transports stand down.

### The advisor on a refused order (issue #1, qol 1.6.2)

Stock's only word that the bank is short comes from `Producer::StartBuild` (0x4b8180)
when it fails: it tests the bank in the order dilithium (with the `EPSILON` margin),
officers (cost above `Team::GetOfficerVacancies`), crew, latinum, metal, biomatter, and
at the first shortfall calls `ComputerVoice::Say(Topic, GameObject *, const Race *)`
(0x45d8c0, `__cdecl`) with the producer and a null race, topic 9, 10, 0x36, 0x37 or 0x38
for dilithium, crew, latinum, metal and biomatter. For officers it asks the team's race
(`Team` +0x244) for `Race::GetInsufficentOfficerEventId` (0x48c120, `__thiscall`) and
passes that to the int overload, `ComputerVoice::Say(int, GameObject *, const Race *)`
(0x45db20). Under `PayOnQueue` a refused order never reaches `StartBuild`, so `h_push`
dropped it in silence; it now makes the same call for the same first shortfall
(`short_of` keeps `StartBuild`'s order, so the line is the one stock would have
spoken). `voice_ok()` checks, before anything is called, that `StartBuild` still pushes
topic 9 at 0x4b826c and calls the three functions at 0x4b826e, 0x4b829a and 0x4b82a0;
if not, refusals stay silent and the startup line says so.

Why this is safe on every node: both `Say`s end in `GameEvent::TriggerEvent(GameObject *,
const Race *)` (0x479b70), which plays only when the object's team is
`Entity::GetUserTeam()` (unless the event is flagged for everyone), and not again within
the event's own interval, timed with `Get_TimeLocal` (wall clock). The sound is picked
with `g_asyncRandom` in `GameEvent::mTriggerRaceSound`; the rest is the chat line and
minimap mark the event carries. Nothing the simulation reads changes and `g_random` is
not drawn, so `Handle_New_Command` running on every node of a network game speaks only
on the machine of the player refused, and repeated clicks are held back by the same
gates as stock's. Measured on the bench (Instant Action, Federation, `--stems voice`):
stock, with a build stalled on dilithium, spoke once in 100 s, and once more per order
added to the stalled queue (three orders 0.15 s apart: one line; 0.5 s apart: two; 2 s
apart: three); the plugin, refusing for crew, spoke one line for three orders 0.15 s
apart and three for three 0.5 s apart -- the same, within the timing of the clicks. Each call writes an `advisor:` line in
`QOLRules.log` naming the topic (-1 for officers) and whether the team is the local one.

Losing the building and saving have a scenario, `qol-pay-on-queue-loss` (a yard destroyed by a
hull hit, handed to another team, saved and loaded; the bank must come back once, never to
3725). It was written against an earlier hook set that also hooked `DestroyShip`, where the
first run found a double refund and `ClearTeam` was made to skip a destroyed producer. This
layer hooks `ClearTeam` only, which stock zeroes the item in progress after, so a destroyed
producer is refunded exactly there and a guard on the destroyed flag would lose that refund.
The scenario has not been run against this layer yet.

Not done: a refit save with paid items loaded in stock loses what was paid; construction ships and
the AI's own orders (if it bypasses the command) are untested.
