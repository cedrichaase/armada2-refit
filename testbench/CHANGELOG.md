# Changelog — testbench

`./a2test`: headless end-to-end runs of the game at any resolution, with plain-text
scenarios, screenshots and reports. It installs nothing into the game: every case runs
on a reflink clone. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The why is in [`README.md`](README.md).

## 2.9.0 — 2026-10-03

### Added
- `testbench/d3dtrace/`: `D3DTrace.asi`, a Direct3D 8 call tracer that goes into bench
  clones only (`--install testbench/d3dtrace`), and `summarize.py`, which groups a
  traced frame's draws by lighting path. A session's `D3DTrace.log` is gathered with the
  other logs.
- `testbench/d3dtrace/sod-bump.py`: puts a clone's models on the engine's dot3 bump path
  (SOD material type 4 → 6 plus a `<texture>bump` height map, `--height flat|highpass`).

## 2.8.1 — 2026-10-03

### Changed
- `scenarios/no-assets.md` checks that `Planets.asi` patched both of its sites (88887e3).

Installs nothing; no game-side change to confirm.

## 2.8.0 — 2026-10-03

### Added
- `GAMEPLAY.md`: how to play a match through the bench (selecting, the build menu,
  placing a structure, mining, the traps), from the first agent that mined resources.
  Every agent step's prompt includes it.
- Agent steps report `lessons`, written into the case's log and appended to
  `agent-lessons.md` in the results directory (af3c97c).

Installs nothing; no game-side change to confirm.

## 2.7.1 — 2026-10-03

### Fixed
- `a2test drive step` failed on every step (`'Case' object has no attribute
  'sessions'`) since scenarios with `Players:` (2.4.0).

Installs nothing; no game-side change to confirm.

## 2.7.0 — 2026-10-03

### Added
- Step `Type what follows "TEXT" in "LOG" of PLAYER`: the word after TEXT in a log
  of that player's game (a join code), typed into this one's.
- `scenarios/multiplayer-online-code.md` (joining by join code, direct path) and
  `multiplayer-online-relay.md` (the same through the server's relay, 5% loss); the
  match fragment is now `_online-setup.md` + `_online-play.md`, with the joiner's typing
  step between them.
- `OnlineServer.log` is collected with the other logs (2f2255a).

Installs nothing; no game-side change to confirm.

## 2.6.0 — 2026-10-03

### Added
- `scenarios/multiplayer-online-match.md`: host, join, chat and a match through
  *Internet – Online* on our own transport, under plain Proton (online 0.3.0), and
  `multiplayer-online-loss.md`, the same with 10% of datagrams dropped. Their steps are
  the fragment `_online-match.md` (6cbeed9).

Installs nothing; no game-side change to confirm.

## 2.5.0 — 2026-10-03

### Added
- `scenarios/multiplayer-online-entry.md`: both players host and join through
  *Internet – Online* (online 0.2.0), then chat both ways (fd40da4).

Installs nothing; no game-side change to confirm.

## 2.4.0 — 2026-10-03

### Added
- Scenario headers `Players:` (one game per player, steps prefixed with the player's
  name) and `Setup:` (a repository script run on each clone before launch), and the step
  `Type this machine's address` (245d1f6).
- `scenarios/multiplayer-two-players.md`: host, join, chat both ways and a 2-player
  match over TCP/IP, with Microsoft's DirectPlay and `Online.asi` in each clone.
- The game's `Online.log` and `Online.ini` are gathered with the other logs.

Installs nothing; no game-side change to confirm.

## 2.3.0 — 2026-10-03

### Added
- `a2test watch`: every live session, view-only, tiled in one Chromium window. The watcher
  starts its own `wayvnc` per session and bridges noVNC's WebSockets to them. Sessions
  come and go on their own. Options `--no-open` and `--port`, and `A2TEST_WATCH_PORT`.

Installs nothing; no game-side change to confirm.

## 2.2.0 — 2026-10-02

### Added
- `scenarios/menu-repaints.md`: Options' "1.1" (main menu and in a mission), an address
  typed into the Manual IP field, and the Technology Tree's fixed pitch, at 21:9
  (menus 4.3.1). Red on menus 4.3.0 (steps 6, 35 and 39), green on 4.3.1. The field's
  first paint is the `manual ip` shot, for the eye.

### Fixed
- `scenarios/multiplayer-name.md` no longer expects the field to read "Player": a clone
  starts with the player's own saved name (`save/shell.set`), so the field is cleared
  before the name is typed.

Installs nothing; no game-side change to confirm.

## 2.1.0 — 2026-10-02

### Added
- `a2test session start --stock-shell embed`: a stock session with the embed-only
  `Menus.asi`, as `Stock shell: embed` gives a scenario, so menus past the main one take
  clicks by hand too.

### Fixed
- `a2test drive click|move --design` crashed (`'Case' object has no attribute 'mod'`).

Installs nothing; no game-side change to confirm.

## 2.0.0 — 2026-09-28

### Changed
- The modded state is called `refit`, after the project: `Mod: refit` in a scenario and
  `--mod refit` on the command line, in place of `remastered`, which is no longer read.
  Every scenario in `scenarios/` is updated.

Installs nothing; no game-side change to confirm.

## 1.12.0 — 2026-09-27

### Added
- `scenarios/hud-mode-switch.md`: switches to 1600x1200 in Graphics Settings in the
  middle of the Federation mission, at 4:3 and 21:9, and measures the HUD, the briefing
  and its text against the 4:3 case. Fails before hud 2.2.0, passes with it (`9873980`).
- `ui.json`: the `display mode` target, the Graphics Settings mode box.

Installs nothing into the game.

## 1.11.0 — 2026-09-27

### Added
- `scenarios/game-setup-labels.md`: Instant Action's Game Setup, with the option labels
  held clear of the minimap frame, before and after the map is revealed (menus 4.3.0; 9838ca6).

Installs nothing into the game.

## 1.10.0 — 2026-09-27

### Added
- The check `Expect "TEXT" is visible inside design X,Y,W,H [and at least N design px
  tall]`: OCR, with the phrase's position and height held to a rectangle in the shell's
  design space.
- The `multiplayer connection` screen in `ui.json`, and `scenarios/multiplayer-name.md`,
  which checks the scaled name field (menus 4.2.0). Red on menus 4.1.0 (steps 6 and 13, at
  1600x1200 and 3440x1440), green on 4.2.0.

Installs nothing into the game.

## 1.9.0 — 2026-09-27

### Added
- `Assets: none` in a scenario's header: the case installs this checkout from stock
  with an empty `A2_DATA`, the game as someone without a texture pack gets it.
- `scenarios/no-assets.md`, which runs that case: every asset layer installs nothing
  and says so, the cutscene proxy passes every movie to the real DLL, and the game
  reaches its own stock main menu with black sides. Passes.

Installs nothing into the game.

## 1.8.1 — 2026-09-27

### Fixed
- A stacked private `./install` finds its builds: the session passes the real
  `A2_DATA` through, resolved before it points `XDG_DATA_HOME` at its own scratch
  directory (which `A2_DATA`'s default hangs off). Without it every private layer
  reported "not built" and the clone stayed stock.

### Changed
- `README.md`'s stacked-install example uses the sibling checkouts, `.` and
  `../armada2-refit-private`. A private worktree now installs the same builds as
  any other private checkout: they live in `$A2_DATA`.

Installs nothing into the game.

## 1.8.0 — 2026-09-27

### Added
- `--install PATH`, repeatable, on `session start` and `run`: the clone starts from
  stock (`a2mod stock`), then each `PATH/install` runs into it in order, so public and
  private checkouts can be tested together without touching the real install. The
  session log records each checkout's commit (`installs=`, `-dirty` if uncommitted).

### Changed
- The game, prefix and Proton come from `a2env.py`.

Checked: public and private worktrees stacked, launched to the main menu at 16:9.

## 1.7.0 — 2026-09-26

### Added
- `_enter-federation-mission` fragment: Federation mission 1's briefing, reopened with
  the camera over unexplored space (minimap click, then the command bar's checkmark).
- `hud-right` coordinate space for steps (`Click at hud-right X,Y`) and `ui.json`
  targets: HUD canvas units anchored top right, at stock's 1600-wide position. Targets
  `objectives` (the checkmark) and `minimap fog`; a target may name any space.
- `Drag from … to …` step (`Session.drag`): press, glide with the button held, release —
  a selection box in a mission.

### Changed
- `hud.md` runs at the Federation briefing over the fog instead of the Borg one in front
  of space, and judges that no thin lines show along panel edges or tile joins. Its third
  HUD region is the command bar (`660,0,140,28`): the right panel is mostly the moving
  ship portrait. The pointer is parked in the fog before the shot. It then selects a
  unit by dragging a box over the fleet and judges the action bar over the fog.

Installs nothing into the game. Confirmed working 2026-09-26.

## 1.6.1 — 2026-09-26

### Changed
- `a2test run --jobs` defaults to 5 (was 3): measured with 5 at once plus the user's own
  game running, at least 5.5 GB stayed available and swap was untouched.

### Fixed
- The pointer lands where it is sent in a mission. `Session.move` sends the difference
  from the last known position as relative motion (new `a2input rel DX DY`); an absolute
  move reached the game as a jump from a stale reference point, so every in-mission move
  ended with the cursor in the top-left corner.
- `hud.md` checks the cursor again, within ±10% (removed in 1.6.0 because it never saw
  the cursor): 0.94–1.02 of stock at every aspect; stock at 21:9 measures 1.79x.

Installs nothing into the game. Confirmed working 2026-09-26.

## 1.6.0 — 2026-09-26

### Changed
- OCR is PaddleOCR's PP-OCRv3 scene-text model through `cv2.dnn` (`bench/ppocr.py`),
  in place of tesseract's three preprocessing passes: 100% of 547 phrase instances in
  86 bench frames against tesseract's 93.6%, ~0.8 s a frame. Its two ONNX models are
  fetched on first use into `~/.cache/a2test/models`, pinned by revision and sha256.

### Removed
- tesseract, and with it the 2x second look of 1.5.1 (`vision.retry_scale`,
  `find_text_in`) and `ocr(scale=, fast=)`. tesseract is no longer a requirement.
- `hud.md`'s cursor check (1.5.1). It never measured the cursor: the game had hidden
  its pointer by the time of the shot, in stock as well, so its 0.96–1.00 "passes"
  were the empty background matching itself. Why the pointer disappears is open
  (`README.md`).

Installs nothing into the game. Confirmed working 2026-09-26.

## 1.5.1 — 2026-09-26

### Changed
- `hud.md` checks the cursor: the pointer is parked in empty space and the region
  around it is matched against stock 800x600, like a HUD panel.

### Fixed
- Text search takes a second look at 2x on frames over 1200 high when 1x misses
  (`vision.retry_scale`): at 3440x1440 the mission list's "Werewolf Pack" read as
  "B er If Pack" twice in a row.

Installs nothing into the game. Confirmed working 2026-09-26.

## 1.5.0 — 2026-09-26

### Added
- `HUD.log` and `HUD.ini` are collected with each case's logs.

### Changed
- The refit prepare step is `hud/install.sh` (`HUD.asi`), the same for every
  case, in place of `ui-widescreen.py`, `cursor-aspect.py` and `ui-font-condense.py`
  run with `--res` per case.

Installs nothing into the game. Confirmed working 2026-09-26.

## 1.4.0 — 2026-09-26

### Added
- `… within N%` on a stretch check: that step's tolerance, in place of `--tolerance`.

### Changed
- `hud.md`: the three font checks pass within ±10% (measured 0.91–0.94 of stock, judged
  fine by the user in game); the HUD regions stay at ±5%.

Installs nothing into the game. Confirmed working 2026-09-26.

## 1.3.0 — 2026-09-26

### Added
- `Stock shell: embed` (scenario header): stock cases keep `Menus.asi` with `Embed=1`
  and nothing else, so the stock menus take injected clicks (without it the Borg
  campaign click never registered, at 1600x1200 or 800x600). `hud.md` and
  `stock-control.md` use it.

### Changed
- `hud.md` measures against stock at **800x600**, the resolution the game was designed
  for; regions are in 800x600 pixels.
- A comparison with no baseline (the reference case failed or never ran) is SKIP with
  the reason, not inconclusive. Judged comparisons are skipped the same way rather than
  judged blind.

### Fixed
- Silent runs: each session gets its own null sink, made Wine's default output
  (`DefaultOutput` under a pre-registered GUID in the clone's `user.reg`), because the
  game crashes at start-up with no audio device. A `pactl subscribe` watchdog moves,
  mutes and stops any session stream that lands elsewhere; streams not yet linked, and
  streams on any `a2test-` sink, are not leaks.
- Prepare runs `--revert` before `ui-widescreen.py` and `cursor-aspect.py`: at 4:3 both
  do nothing, so a 4:3 case measured the install's 21:9 layout (HUD 0.56x).
- Stock clicks are scaled to the screen: the 800x600 shell mode fills the output.
- OCR reads frames up to 700 px tall at 4x; 2x missed 8 px shell text at 800x600.
- A shot belongs to a case by path, not string prefix (`1600x1200-stock` matched
  `1600x1200`).
- A case cut short by Ctrl-C/SIGTERM is an error, not a PASS.

Installs nothing into the game. Measured run 20260926-030021: stock 800x600 PASS;
refit HUD within 2% of stock at 4:3, 16:10, 16:9 and 21:9, font 0.91-0.94x.
Confirmed working 2026-09-26.

## 1.2.1 — 2026-09-26

### Changed
- Judged and agent steps run on Sonnet by default (`--model sonnet`), not the `claude`
  CLI's default (Opus here). `A2TEST_MODEL` still overrides it; empty restores the CLI's.

Installs nothing into the game. Confirmed working 2026-09-26.

## 1.2.0 — 2026-09-26

### Added
- `a2test run --jobs N` (`-j`, `A2TEST_JOBS`): up to N cases at once, default 3. A
  scenario's reference case runs first; its other cases are queued once it is done.
  Ctrl-C / SIGTERM stops every running game and waits for its teardown.

### Fixed
- Silent runs: `winepipewire.drv` disabled alongside pulse and alsa. (Superseded in
  1.3.0: with no driver at all the game crashes at start-up.)
- Session ids gain a per-process counter (timestamp and pid collide across threads);
  VNC ports are claimed under a lock; `a2input` is built once before any case starts.

Installs nothing into the game. Confirmed working 2026-09-26.

## 1.1.0 — 2026-09-26

### Added
- `Reference: <aspect> <mod>` and `compared with stock 4:3`: a reference can name its
  mod state. A reference in the other state runs first as its own case
  (`<res>-stock`); the scenario's own 4:3 case is then measured, not skipped.

### Changed
- `hud.md` measures the HUD and font against **stock** 4:3, the shape they were drawn
  for, instead of refit 4:3. The judge brief no longer excuses a condensed font.

Installs nothing into the game. Confirmed working 2026-09-26.

## 1.0.0 — 2026-09-25

### Added
- `a2test run | check | list | session | drive`: clone, per-resolution prepare,
  headless sway + Xwayland, umu launch, `a2input` virtual pointer/keyboard, `grim`
  capture, teardown; `index.html` / `report.md` / `results.json` per run.
- The scenario grammar (`scenarios/README.md`): deterministic steps, measured checks
  (OCR, stretch against the 4:3 reference, black bars, flat areas, logs, crash),
  judged steps and agent steps through `claude -p`.
- Scenarios: `main-menu`, `campaign-screen`, `borg-mission-1` (and its prose twin),
  `admirals-log` (menus 2.0.1), `escape-menu` (menus 2.0.2, 2.1.0), `hud` (HUD and
  font, at the Borg briefing), `stock-control`; the `_enter-borg-mission` fragment
  and `Include`.
- `ui.json`: screen signatures and click targets in 800x600 design space.

Installs nothing into the game. Validated by a bench run at 4:3, 16:10, 16:9 and 21:9.
Confirmed working 2026-09-26.
