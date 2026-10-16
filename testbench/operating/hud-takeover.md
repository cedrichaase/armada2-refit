# The HUD's panels on the bench (UI.asi's [HUD])

What was learned driving a mission with the UI's own HUD panels (`ui/HUD.md`), for the
agents after.

- **Getting UI.asi with a panel on into a clone.** `session start --no-launch` (with
  or without `--install <checkout>`), then `ui/bench-asi.sh <state.json> --no-grid
  HUD.Bar=1`, then `drive launch`. `--no-grid` takes `GridLayout.asi` out: an
  `--install` of `./install` puts it in, and the UI's bar stands down while it is
  loaded. To try a new build, `drive quit`, run `bench-asi.sh` again (it builds), and
  launch again: the session and its clone survive the quit.
- **Instant Action by hand**: `drive launch`, then `step "Wait for the main menu."`,
  `step "Click the instant action cube."`, `step 'Wait for "GAME SETUP".'`, `click X Y
  --design` for 124,115 / 124,143 / 246,95 / 246,123 a second apart, `step 'Click
  "LAUNCH".'`, and about 10 s. Put the sequence in a small script: the agent sandbox
  refuses long shell lines that mix loops, variables and `bash -c`.
- **Where things are at 1280x720** (Borg, Warzone, the camera as the match starts):
  the first Assembler at 640,262, the Nexus at 755,320. The second Assembler (about
  680,280) drifts a little from run to run and is not a reliable click. With `HUD.asi`
  the info panel spans x 376..904; the UI's bar sits above it (its Q cell at about
  400,482 for an Assembler, 400,411 for the Nexus, whose panel is taller), 45 px per
  cell.
- **A good place for a station**: 900,300 places the Assembler's mining station (S in
  its build list); 1000,200 is refused ("Cannot build at specified location").
- **The bar's log is the evidence**: `UI.log` writes each layout once per change
  (`bar: menu=2 mode=1 | Q=bbase ... T=cancel(off) ... G=back`) and each press (`bar:
  click build`, `bar: key S=bmining`). For one frame after a press the menu number
  changes before the buttons do; the bar lays out by the buttons' menu, so the log
  does not show a mixed line.
- **UI.log and UI.ini are gathered into the report** (`logs/`) since this work;
  `Expect "UI.log" contains` reads it from the clone while the game runs, and a session
  can copy it from `~/.cache/a2test/sessions/<ID>/game/UI.log` before `session stop`.
- **Quitting from a mission hangs on a black screen** (2026-10-16, 1280x720): after
  Exit to Windows and Yes the game stays up with a black screen until the bench stops
  the prefix, so `Quit the game` fails. Seen in `ui-hud-bar` (with and without
  `--install`), in another agent's `ui-style` run, and reproduced in a session **with
  UI.asi taken out of the clone**, so it is not the UI's. Two quits in an earlier
  session the same night went through. Not investigated further.
