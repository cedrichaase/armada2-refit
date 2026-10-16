# The whole game with the UI's defaults (`ui-whole-game.md`)

What an agent driving the game as `./install` now gives it (every part of the UI on, no
`HUD.asi`, `GridLayout.asi` or `Menus.asi`) needs to know. Learnt 2026-10-16.

## Getting there

- `./a2test session start --res 1280x720 --install . --no-launch`, then
  `testbench/scene/bench-setup.sh <state.json> ending` for `Scene "succeed"`, then
  `drive launch`. **`drive launch ""` still launches with `-nointro`** (the CLI turns an
  empty argument into the default); a scenario's `Launch the game with ""` does launch
  without it. In a session, the main menu's REPLAY INTRO plays the intro (Esc skips it
  back to the main menu).
- Scenarios that test the superseded plugins themselves put them back with
  `Setup: ui/bench-legacy.sh hud|grid|menus` (the UI's part switched off).
- Do not copy a new `UI.asi` over the one a running game has loaded: stop the game first.

## Where things are, at any aspect

- The main menu's faded art still takes the `ui.json` targets ("Click the single player
  emblem", "the instant action cube", "the options panel"): use those, not the column's
  pixel positions, so one scenario runs at every resolution.
- The centred specs (Save Game, Load Game) map 1:1 onto the `design` space: at 1280x720
  Save Game's name field is `design 450,506` and its Save / Load button `design 400,532`.
- The HUD of ours is in units of the screen's height: `hud` (left-anchored) and
  `hud-right` coordinates land on it at every aspect. The minimap's middle is about
  `hud 187,1013`; a drag from `hud 400,300` to `hud-right 1200,800` covers the middle of
  the screen at 4:3, 16:9 and 21:9.

## What happens

- The bench's profile starts in interface mode 1 (no unit view): Ctrl+I three times
  gives mode 0, and UI.log says "unit view: shown". The mode is kept in the clone's
  `ARMADA.PRF`, so a second round in the same session cycles past it.
- Federation mission 1: the loading screen of ours, then about 70 s of opening cutscene
  (view 2, no HUD), then the briefing. After Abort Mission and Yes the Admiral's Log
  comes up over the mission; its Done leads to the single-player screen (Previous
  Menu for the main menu; once, in a session that had loaded a save before, to the
  campaign's mission list). After `Scene "succeed"` in the campaign the
  Log's Done led to the single-player screen; after an Instant Action match to the main
  menu.
- Instant Action's Game Setup defaults (Computer AI:Medium, a random side for player 1)
  launch with LAUNCH; the side differs per run.
- Minimap's left click moves the camera; a right click orders the selection
  ("minimap: order 0x2b" in UI.log).

## Traps

- **A hang that looks like a black screen** (the system arrow on black, the main
  thread at 100%) is the game's own exception handler walking a broken call stack
  after a crash: `exception.txt` is not written. `gdb -p <pid> -batch -ex "bt 10"`
  shows `DebugCallStack_DumpFrom` / `DebugException_Execute` (name the frames from
  `armada2.map`). Seen once, loading a save from the in-mission menu after keys were
  typed into the mission; not reproduced since. `testbench/scenarios/ui-whole-game.md`
  goes through the same path.
- A gdb attached with SIGSEGV stopping catches Wine's own handled faults too.
