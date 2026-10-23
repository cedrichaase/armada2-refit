# The UI graph

Which screens the game has, and how to get from one to another: a map agents walk
instead of re-reading the routes in `operating/*.md`, and grow as they work.

    a2test drive --session ID where            # which screen this is; its known ways out
    a2test drive --session ID goto briefing    # walk there, every hop confirmed
    a2test graph fold                          # the observations -> testbench/uigraph.json
    a2test graph show ["single player"]        # the edges (from one screen)
    a2test graph path main briefing            # the shortest known way
    a2test graph frontier [SCREEN]             # labels seen on a screen, not yet followed
    a2test graph unknown                       # actions that landed on a screen ui.json does not name

In a scenario: `Go to the briefing`.

## What it is made of

- **Screens** are `ui.json`'s, each identified by text OCR must find on it (`text`). A
  screen's identity is always **what is on screen**, so the graph works for any game
  the bench can see and click. Where several match, the one with the longest signature
  wins ("Cancel" alone is on many screens).
- **Edges** are an action on a screen and where it went: a click on a `ui.json` target,
  on a label found by OCR, on a design-space point, or a key. One edge per screen,
  action and front end (`variant`: `stock`, `ui` for UI.asi's menu host, `menus` for
  Menus.asi), since the shell looks and clicks differently under each. An edge points
  where it went **the last time**, with how many times running it went there (`n`) and
  where it went before, if that differed.
- **Observations** are what the graph is folded from: one JSON line per action an agent
  took and what it saw, appended under a lock to `$A2_DATA/uigraph/observations.jsonl`.
  Every checkout and job shares the file, and none edits another's lines.
  `a2test graph fold` reduces them to `testbench/uigraph.json`, which is committed:
  screen names, labels and log lines, no pictures. Do not hand-edit it; fold again.
  Each observation names the session and a kept frame, so an odd edge can be traced
  to the shot it came from.
- **The frontier** is every short label seen on a screen that no edge from it has
  followed yet: what an agent with time to spare, or a mapping pass, tries next.
  OCR reads painted text too, so it is a list of candidates, not of buttons. A label
  must read the same on at least 60% of the looks at its screen, which drops art that
  OCR reads differently each time (the main menu's logo came back as ARMA, ARMY, ARY
  and ARM). Art that reads the same every time ("OrAR" on the single-player screen)
  stays, and a mapping pass rules it out by clicking it once.
- **Unknown** lists actions that landed on a screen no signature matches, with the
  labels it showed. Naming it is adding it to `ui.json`'s `screens` with a signature
  that picks it out (a few labels no other screen has).

## Seen first, logged when earned

A hop is confirmed by looking: after the action, the bench looks until the screen is a
known one (`ui.json`'s `timeout`, `skip` and `settle` apply, as for `Wait for the …`).
That is what makes the graph portable and what keeps it honest, and it is slow: about a
second an OCR pass.

The **fast path is a log line, and an edge has to earn it**. Every observation keeps the
lines the game's logs (`UI.log`, `Menus.log`) got after the action. When the last three
arrivals of an edge confirmed on sight (`EARN` in `uigraph.py`) all wrote one line, with
numbers and handles taken out, and no observation that landed anywhere else wrote it,
the fold attaches it to the edge as its `signature`. `goto` then takes that hop as soon
as the line is written, plus the screen's `settle`, and records it with
`evidence: log`. Log arrivals never count towards earning.

A signature is recomputed at every fold from the latest visual arrivals, so it is lost
as soon as one of them did not write the line. `goto --verify` walks on sight even
where a line is earned and records whether the line agreed: that is how a change to a
plugin's logging, or to the screens, takes a signature away instead of leaving `goto`
trusting a line that no longer means what it did. Some hops never earn one: closing a
nested dialog in UI.asi's host writes nothing (the screen under it was never reopened),
so those stay visual, which is correct, only slower.

This is the rule `scenarios/README.md` sets for waits ("A log line says what a plugin
did, not what is on the screen"). Whether a screen *renders* right, and where its
buttons are, is never taken from a log. Those are OCR, judged steps and shots.

## How it grows

**As a side effect of work.** `a2test drive click`, `click-text` and `key` record an
observation whenever they start on a known screen (`A2TEST_GRAPH=0` turns it off). A
click on a known screen reads it before and watches after: about 2.5 s more than a bare
click when nothing changes, and until the next screen is recognised (at most 8 s) when
something does. Off a known screen (in a mission's 3D view) only the look before is
paid. A raw click is recorded as the `ui.json` target it hit, else as its design-space
point with the nearest label as a hint. `drive where` records a visit, which feeds the
frontier. `goto` records every hop, so walking the graph keeps confirming it. Scenarios
record nothing yet.

**When an agent writes a new screen** (a menu entry, a dialog of a plugin), it adds the
screen to `ui.json` with a signature in the same change, and walks to it once with
`drive click` so the fold has the edge. A log line its own code writes still has to be
seen to agree three times before `goto` relies on it.

**Fold when you are done**, after a session that added observations: `a2test graph fold`
and commit `testbench/uigraph.json` with your other changes (bench data, no changelog
entry of its own). Parallel worktrees folding the same store produce the same file up
to newer observations, so a conflict is resolved by folding again.

**A mapping pass** (planned, `a2test map`): one session that expands the frontier
breadth-first from where it starts, walking back with `goto`, until the frontier is
empty or a budget is spent, writing the same observations. Run it when there is no
graph (another game, a fresh `ui.json`), or after a change to a large part of the menus,
scoped to that part (`--from "single player"`). Many edges going stale at once is the
hint that one is due. `ui.json`'s `avoid` lists actions that end the game or destroy
something (Exit, Exit to Windows, Delete). They are recorded when an agent takes them
by hand, but neither `goto` nor a mapping pass ever takes them.

## The adapter

`bench/uigraph.py` knows no game. It reaches Armada II through `bench/uigraph_a2.py`:
`look` (a screen grab and its OCR), `act` (target, text, design point, key), `mark` and
`new_lines` (the logs since the action, the bench's own log marks), `variant` and
`context`. Design space, the Menus.asi/UI.asi mapping, and which logs announce a screen
are all the adapter's. A bench for another game supplies another adapter and its own
`ui.json`, and starts with an empty graph and a mapping pass.

## Status

Built: recording from `drive`, `where`, `goto` (and `Go to the …`), `graph fold`/`show`/
`path`/`frontier`/`unknown`, and `bench/uigraph_selftest.py`, which checks folding,
earning and losing a signature, the frontier and walking against a pretend game. On the
bench (2026-10-23, 1280x720), recording, folding and `goto` both ways between the main
menu and the single-player screen worked on sight, about 3 s a hop. The log fast path
has been exercised only by the self-test. That run's install had UI.asi standing down
(no Direct3D 9 device), so the shell was stock and nothing logged its screens. The first
graph is folded from observations made on a working install. That test's observations
are in `$A2_DATA/uigraph/archive/`, not in the store.

## Open

- **One graph for stock and refit, edges per front end.** Screens are shared: "single
  player" is one place in both. What an edge clicks, and the log line it may earn,
  belong to its `variant`, which the adapter reads off what the plugins *logged*, not
  which are installed. In the test above, a refit install whose UI.asi stood down was
  correctly a stock shell. Screen signatures are not per variant yet. Where UI.asi
  draws a screen with other text than stock, its `ui.json` entry will need `text` per
  variant.
- **The mission is not a node yet.** In-mission state is not one screen. The HUD differs
  by faction, and the game is in a mission rather than on a screen, so Escape into the
  in-mission options menu is recorded only once a mission view has a signature.
  Scenes and `operating/` recipes still cover gameplay.
- **Lists are one edge per label.** "Invasion" and "Werewolf Pack" are separate edges from
  the mission selection. A parameterised edge (`mission=NAME`) waits until enough of
  them exist to show the pattern.
- **A slow transition observed by `drive`** (a click that starts a mission load) is
  recorded as landing on an unknown screen, the loading screen, after the 8 s look.
  `goto` waits by the target screen's `timeout` instead.
