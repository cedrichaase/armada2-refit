# Scenarios — the step grammar

A scenario is a markdown file here. `./a2test run <name>` runs it once per resolution;
`./a2test check <name>` shows how each step will be carried out, without launching.

```markdown
# Title of the test

Resolutions: 4:3, 16:10, 16:9, 21:9       (aspects or WxH; "all" = these four)
Mod: refit                           (or stock: a2mod stock, on the clone)
Launch: -nointro a2_borg01                (Armada2.exe arguments; default -nointro)
Reference: 4:3 stock                      (what "compared with" means; default 4:3 in
                                           this scenario's own mod state)
Stock shell: embed                        (stock cases only: keep Menus.asi, Embed=1 only)
Assets: none                              (install this checkout with an empty A2_DATA:
                                           the game as without a texture pack)
Players: host, joiner                     (one game per player; see "Several players")
Setup: online/bench-asi.sh --loss 10      (a script of this repo, run on each clone
                                           before launch, with its session state file
                                           and then any words after the script's name)
Sheet: 1 ship, 6 ships, 30 ships          (shot names: <scenario>/sheet.png in the run,
                                           a row per case and a column per shot)
Clock: bench                              (the plugins' animation clock on the simulation's
                                           time: ../README.md, "A frame that repeats")
Timeout: 12 min

Any prose is description.

1. Launch the game.
2. ...
```

Every numbered or bulleted line is one step. Quotes may be straight or curly, and a
leading "Then" or "And", or a trailing full stop, is ignored. A `## ` heading among the
steps names a phase (`## Main menu`, `## Mission HUD`): the bench dashboard shows how far
each phase has got. Without headings it makes one phase per screenshot.

## Deterministic steps

| Step | What it does |
|---|---|
| `Launch the game` / `… with "ARGS"` / `… into mission "a2_borg01"` | umu launch; then proves the game reads its clone (`/proc` scan) |
| `Quit the game` | through the game's menus (Esc, Exit to Windows, Yes; or the main menu's Exit). The game ignores WM_CLOSE. Fails if it has to force it |
| `Stop the game` | kills the prefix |
| `Wait 5 seconds` | the game must stay alive meanwhile |
| `Wait for "TEXT"` / `Wait up to 60 seconds for "TEXT"` | OCR poll |
| `Wait for the main menu` / `… the single player screen` / `… the briefing` / `… the options menu` / `… the admiral's log` / `… the mission selection` | OCR signature from `../ui.json` |
| `Click the single player emblem` (any `ui.json` target) | design coordinates, mapped the way Menus.asi maps them, unless the target names another space (`objectives`, `minimap fog` are HUD targets) |
| `Click "TEXT"` / `Right-click …` / `Double-click …` | OCR, then click the centre |
| `Click at 100,200` / `… at design 400,300` / `… at hud 25,27` / `… at hud-right 1500,26` | screen pixels, 800x600 shell space, HUD canvas (1200 high, top-left), or HUD canvas anchored top right at its stock 1600-wide position |
| `Move the mouse to …` / `Hover over …` | same four coordinate spaces |
| `Drag from hud 400,300 to hud 1400,900` | press, glide with the button held, release (a selection box); either end in any of the four spaces |
| `Right-drag from 960,500 to 760,500` | the same with the right button: pans the map in a mission |
| `Press Escape` / `Press ctrl+s` / `Press Return 3 times` | virtual keyboard; xkb key names |
| `Type "TEXT"` | |
| `Type this machine's address` | the source address of the default route: what the game shows as "Local IP Address", and where every player of a case is |
| `Type what follows "TEXT" in "LOG" of PLAYER` | the word after TEXT on the last line of that player's log that has it (`of PLAYER` left out: this game's), waiting up to 30 s for it: how a joiner types the host's join code |
| `Take a screenshot called "NAME"` | kept in the report; same-named shots are compared across resolutions |
| `Dump the frame called "NAME"` | Render3D.asi's targets (PFM) and UI.asi's presented frame (PPM), read back inside the game, into `dumps/NAME/` with PNG previews (`../README.md`, "Frame dumps"); fails when no plugin dumped |
| `Note "TEXT"` | a line in the log |
| `Mark the display context "NAME"` | Display8's calls from here on count under that context (`Display8.mark`; `../../display/README.md`, "The inventory"); fails without Display8 installed |
| `Wait up to 240 seconds for "Scene.log" to say "TEXT"` | polls a log beside `Armada2.exe` for a line with TEXT written since the last action (a launch, click, key, typing, drag or Scene command; each launch deletes the old logs), and goes on as soon as one is. At the time limit a line from before that action passes; none at all is a failed check |
| `Wait up to 50 seconds for the logs the next steps expect` | the fixed wait before a run of `Expect "LOG" contains "TEXT"` steps, ended as soon as every line they expect has been written since the last action; when one has not, the whole time, as `Wait 50 seconds` would. It checks nothing: the Expect steps after it do. See "Waits and checks" |
| `Scene "select yard1 yard2"` | commands to `Scene.asi` (`../scene/README.md`), `;` between several; a refused command fails the step. Needs `Setup: testbench/scene/bench-setup.sh <scene>` |
| `Expect scene "selection" answers "TEXT"` / `… not answer …` | that command's own answer, not the whole `Scene.log` (a check) |
| `Include "_enter-borg-mission"` | that file's steps, in place, at parse time. Files starting with `_` are fragments and are not listed |

## Waits and checks

A wait decides **when** the next step runs; a check decides **whether** the case passes.
Keep them apart: a wait never stands in for a check, and a fixed wait is the last resort,
because it costs its whole time in every case at every resolution even when the game was
ready long before.

Pick the wait by what the next steps need:

| The next steps need | Wait with | Then check with |
|---|---|---|
| something a plugin does (the scene is up, an effect drawn, a key handled) | `Wait up to N seconds for the logs the next steps expect` | `Expect "LOG" contains "TEXT"`, one per line |
| a log line the steps after it read (a join code) | `Wait up to N seconds for "LOG" to say "TEXT"` | whatever follows |
| text or a screen to show | `Wait up to N seconds for "TEXT"` / `Wait for the main menu` (OCR) | the wait is the check: it fails at the limit |
| the result of a Scene command | nothing: `Scene "…"` returns on Scene.asi's answer | `Expect scene "…" answers "…"` |
| the next frame to be drawn, before a screenshot or OCR check | a short fixed `Wait 2 seconds` | the screenshot, OCR or judged step |
| time to pass in the game (a battle, an effect developing, a cooldown) | a fixed `Wait N seconds`: here the time is the point | the screenshot or check after it |

The rules behind the table:

- **A log line says what a plugin did, not what is on the screen.** Waiting on one is a
  signal to go on; where a screenshot, OCR or a judged step follows, that still decides.
  Don't replace a visual check with a log check because it is faster.
- **Only lines written since the last action count** (a launch, click, key, typing, drag
  or Scene command), so a line an earlier step left cannot end the wait for the next.
  A line written as a plugin loads ("module effects: on") is older than any click; after
  one, a wait on it runs its whole time. Wait on something the action causes.
- **A log wait can end before the frame is drawn**, since plugins log while they build
  it. Keep the short fixed wait between an input and a screenshot or OCR check.
- **Set N to the slowest you have seen, with margin.** A log wait that passes costs only
  the time it took, so a generous limit is free; a fixed wait costs all of it.
- **Need a wait nothing logs for?** Add the line to the plugin (or to Scene.asi), written
  when the thing has happened, and wait on it. That is better than guessing a time.
- **Prove a wait waits on the right thing**: run the scenario and read the step's detail
  in `steps.jsonl`. "all 3 written after 0.1 s" right after a launch is right when the
  launch step itself took 12 s; "waited the full 45 s" means the line never came after the
  action and the wait did nothing for you.

## Several players

`Players: host, joiner` runs one game per name, each on its own clone, prefix,
display and input, under `<case>/<player>/`. A step that starts with a player's name
runs on that player's game (`Host: Click "Create Game"`); a step without one runs on
each game in turn and stops at the first that does not pass (`Launch the game`,
`Expect no crash`). The steps run in the order written, so a wait on one player's
screen is how the scenario waits for something the other one did. The case keeps one
step log, each line tagged with its player. Such a case is still one job for
`--jobs`, but runs as many games as it has players. `multiplayer-two-players.md` is
the example. The one time two such cases started together (four games), one game's
audio reached the real output before the bench's guard stopped it (run
20261003-165008). The other case passed, and that one passed when run again alone.

## Checks (measured)

| Check | How |
|---|---|
| `Expect "TEXT" is visible` / `… is not visible` | OCR (PP-OCR), fuzzy match |
| `Expect "TEXT" is visible inside design X,Y,W,H` / `… and at least N design px tall` | OCR; the phrase's centre must fall in that 800x600 design rectangle, mapped as Menus.asi maps it, and its box, mapped back, must be N design px tall. Catches a control left unscaled in the corner |
| `Expect the main menu` (any `ui.json` screen) | OCR signature |
| `Expect the game is still running` / `Expect the game to have exited` | process table |
| `Expect no crash` | `exception.txt` untouched |
| `Expect the engine to draw nothing in "NAME"` | no `Draw*` call from `Armada2.exe` in that display context (`Display8.inventory.tsv`, rewritten every two seconds); lists the call sites when there are some |
| `Expect "Menus.log" contains "TEXT"` / `… not contain …` | any log beside `Armada2.exe`, or one of the bench's |
| `Check that the screenshot is not stretched compared with 4:3` | template match of the reference shot over horizontal scales; vertical fixed at H/H_ref |
| `… the hud is not stretched compared with 4:3 in region X,Y,W,H` | same, for a patch in reference pixels |
| `Check that "TEXT" is not stretched compared with 4:3` | OCR finds the phrase in the reference shot; the template match measures that patch here |
| `… within 10%` (after any stretch check) | that step's tolerance, in place of `--tolerance` (default ±5%) |
| `… is stretched …` | the control form: passes only if it IS stretched |
| `Expect no black bars` / `Expect black bars` | bounding box of non-black pixels |
| `Expect no large flat grey areas` | untextured non-black blocks (don't use on a map: fog of war is flat grey) |
| `Expect shots "first" and "second" to match` / `… within 0.5` | the newest shot of each name, pixel by pixel: mean absolute difference (0..255, every channel) at most the tolerance (default 0.5); the largest difference and the share of pixels that differ are reported |

### The HUD's layout

These read the panels' rectangles from UI.asi itself, never from the screenshot: with
`[HUD] LayoutLog=1` (`ui/bench-asi.sh ... HUD.LayoutLog=1`) it writes each new layout
to UI.log (`ui/HUD.md`, "The layout log"). Each check takes the last `layout:` line at
the case's resolution, waiting up to 8 s for one, and keeps a screenshot with every
panel drawn on it, in red those that failed.

| Check | How |
|---|---|
| `Take a layout screenshot called "NAME"` | the named shot, and `NAME-layout.png` with the panels drawn on it (what `Sheet:` shows) |
| `Expect the HUD's panels not to overlap` | fails when two rectangles share more than a pixel on both axes |
| `Expect the HUD's panels at the bottom to rise at most 45% of the screen's height` | every panel whose bottom edge is in the lower half: its top edge at or below that share of the height, counted from the bottom |
| `Expect the HUD's panels to cover at most 30% of the screen` | the area of the panels' union, over the screen's |

`ui-hud-layout.md` runs them at 4:3, 16:9 and 21:9 with one, six and thirty ships
selected.

A comparison step uses this case's latest screenshot and the reference case's shot with
the same name, so take a named screenshot first. `a2test run` runs the reference case
first, adding it if the scenario doesn't list it. In the reference case itself these
checks are SKIP.

A reference can name a mod state: `Reference: 4:3 stock`, or in one step, `compared
with stock 4:3`. Anything whose target shape is the original game's (the HUD, the
cursors, the font) should use stock 4:3, since that is how it was drawn to be seen. A
stock reference is then a case of its own (`1600x1200-stock` in the results), run
first. The scenario's own 4:3 case runs as well and is measured against it rather than
skipped. In a reference-only case, judged steps are SKIP; it only supplies screenshots.

If the reference case fails before it takes the shot, the comparisons that needed it
are SKIP too, and each one says why ("the stock 4:3 reference case failed at step 6").
A missing baseline is the reference case's failure, not an unclear result in this case.
Judged steps that ask for a comparison are skipped the same way rather than sent to the
judge with nothing to compare against.

## Baselines (pixel diff)

Frames kept in `$A2_DATA/baselines/<B>/` and compared pixel for pixel
(`../README.md`, "Baselines", has what each number means).

| Step | What it does |
|---|---|
| `Record the screenshot "NAME" as baseline "B"` | copies this case's latest frame of that name into baseline B, with the case's commits, resolution and scene; replaces a shot of that name. `the dump "NAME"` finds a `.ppm`/`.pfm` dump of that name in the case |
| `Record every screenshot as baseline "B"` | the same for every named screenshot the case has taken so far |
| `Expect the screenshot "NAME" matches baseline "B"` | RMSE, max, pixels over 2/255 and PSNR against the baseline; with no tolerance, no pixel may be over 2/255. The heat map and a side by side go into the case's `baseline/` and the report |
| `… within RMSE 0.5` | passes at an RMSE (in 1/255) up to that |
| `… with at most 0.1% of pixels over 4` | at most that share of pixels with a channel more than 4/255 off. Both tolerances may be given, RMSE first |

A missing baseline, or a baseline without that shot, fails the step and names the step
or command that records it. A baseline holds one size of frame: a scenario run at
several resolutions needs a baseline per resolution (`"pre-gpu-720"`), or the
comparison fails on the size.

## Judged and delegated steps

- **`Check that …` / `Verify …` / `Make sure …`** that is none of the above is *judged*:
  Claude (`claude -p`) reads the screenshot and the same-named reference shot, and
  answers pass / fail / inconclusive with its reasons. With `--no-claude` it becomes
  REVIEW, and a person decides from the screenshots in the report.
- **Anything else** is handed to an *agent*: `claude -p` driving the game through
  `a2test drive` until the step is done, or it reports failure. Every action it takes
  lands in the case's log. Its prompt includes [`../GAMEPLAY.md`](../GAMEPLAY.md), what
  earlier agents learnt about playing a match, and it reports its own `lessons`: they go
  into the case's log and are appended to `agent-lessons.md` in the results directory,
  to be folded into `GAMEPLAY.md` once they hold up.

Prose is fine for a first draft. Rewriting steps into the measured forms makes a
scenario cheaper, faster and repeatable.
