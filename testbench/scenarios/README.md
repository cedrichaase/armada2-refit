# Scenarios — the step grammar

A scenario is a markdown file here. `./a2test run <name>` runs it once per resolution;
`./a2test check <name>` shows how each step will be carried out, without launching.

```markdown
# Title of the test

Resolutions: 4:3, 16:10, 16:9, 21:9       (aspects or WxH; "all" = these four)
Mod: remastered                           (or stock: a2mod stock, on the clone)
Launch: -nointro a2_borg01                (Armada2.exe arguments; default -nointro)
Reference: 4:3 stock                      (what "compared with" means; default 4:3 in
                                           this scenario's own mod state)
Stock shell: embed                        (stock cases only: keep Menus.asi, Embed=1 only)
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
| `Click the single player emblem` (any `ui.json` target) | design coordinates, mapped the way Menus.asi maps them |
| `Click "TEXT"` / `Right-click …` / `Double-click …` | OCR, then click the centre |
| `Click at 100,200` / `… at design 400,300` / `… at hud 25,27` | screen pixels, 800x600 shell space, or HUD canvas (1200 high, top-left) |
| `Move the mouse to …` / `Hover over …` | same three coordinate spaces |
| `Press Escape` / `Press ctrl+s` / `Press Return 3 times` | virtual keyboard; xkb key names |
| `Type "TEXT"` | |
| `Take a screenshot called "NAME"` | kept in the report; same-named shots are compared across resolutions |
| `Note "TEXT"` | a line in the log |
| `Include "_enter-borg-mission"` | that file's steps, in place, at parse time. Files starting with `_` are fragments and are not listed |

## Checks (measured)

| Check | How |
|---|---|
| `Expect "TEXT" is visible` / `… is not visible` | OCR (three preprocessing passes), fuzzy match |
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
