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
Timeout: 12 min

Any prose is description.

1. Launch the game.
2. ...
```

Every numbered or bulleted line is one step. Quotes may be straight or curly, and a
leading "Then" or "And", or a trailing full stop, is ignored.

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
| `Press Escape` / `Press ctrl+s` / `Press Return 3 times` | virtual keyboard; xkb key names |
| `Type "TEXT"` | |
| `Type this machine's address` | the source address of the default route: what the game shows as "Local IP Address", and where every player of a case is |
| `Type what follows "TEXT" in "LOG" of PLAYER` | the word after TEXT on the last line of that player's log that has it (`of PLAYER` left out: this game's), waiting up to 30 s for it: how a joiner types the host's join code |
| `Take a screenshot called "NAME"` | kept in the report; same-named shots are compared across resolutions |
| `Note "TEXT"` | a line in the log |
| `Include "_enter-borg-mission"` | that file's steps, in place, at parse time. Files starting with `_` are fragments and are not listed |

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
| `Expect "Menus.log" contains "TEXT"` / `… not contain …` | any log beside `Armada2.exe`, or one of the bench's |
| `Check that the screenshot is not stretched compared with 4:3` | template match of the reference shot over horizontal scales; vertical fixed at H/H_ref |
| `… the hud is not stretched compared with 4:3 in region X,Y,W,H` | same, for a patch in reference pixels |
| `Check that "TEXT" is not stretched compared with 4:3` | OCR finds the phrase in the reference shot; the template match measures that patch here |
| `… within 10%` (after any stretch check) | that step's tolerance, in place of `--tolerance` (default ±5%) |
| `… is stretched …` | the control form: passes only if it IS stretched |
| `Expect no black bars` / `Expect black bars` | bounding box of non-black pixels |
| `Expect no large flat grey areas` | untextured non-black blocks (don't use on a map: fog of war is flat grey) |

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

## Judged and delegated steps

- **`Check that …` / `Verify …` / `Make sure …`** that is none of the above is *judged*:
  Claude (`claude -p`) reads the screenshot and the same-named reference shot, and
  answers pass / fail / inconclusive with its reasons. With `--no-claude` it becomes
  REVIEW, and a person decides from the screenshots in the report.
- **Anything else** is handed to an *agent*: `claude -p` driving the game through
  `a2test drive` until the step is done, or it reports failure. Every action it takes
  lands in the case's log.

Prose is fine for a first draft. Rewriting steps into the measured forms makes a
scenario cheaper, faster and repeatable.
