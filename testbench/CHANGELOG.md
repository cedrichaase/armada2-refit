# Changelog — testbench

`./a2test`: headless end-to-end runs of the game at any resolution, with plain-text
scenarios, screenshots and reports. It installs nothing into the game: every case runs
on a reflink clone. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The why is in [`README.md`](README.md).

## 3.78.2 — 2026-10-30

### Added
- Scene `nebula-cerulean` (a Galaxy at a Big Cerulean Nebula, `mnebula10`) and the
  scenarios `display-passthrough-nebula-forward` / `display-passthrough-nebula`: every
  render3d module off, the engine's map puffs on the forward chain recorded as baseline
  `passthrough-nebula` at ticks 200, 202 and 220, and the stub with `Passthrough=1`
  held to it (RMSE 1.5), the paused frame to itself.

Installed, not yet seen in game. (be4975d)

## 3.78.1 — 2026-10-30

### Changed
- `display-claims`' "alone" leaves out `UI.asi` as well as `Render3D.asi`
  (`display/bench-claims.sh alone`) and checks Display8's mode-request line; it no
  longer needs the UI's raise of the shell's mode.

Installed, not yet seen in game. (c6d6189)

## 3.78.0 — 2026-10-30

### Changed
- The plugins' logs are read from the clone's Wine prefix, `drive_c/users/*/AppData/Local/armada2-refit/logs/`, with the game directory as the fallback for older builds: `Session.game_log`, `log_path`/`log_dirs`, the log waits (`mark_logs`, `new_log_lines`), `collect_logs` (into `logs/` of the results as before), `Display the mark`, and the launch's clean-up of the previous logs in both places. `gpu/replay.sh`, `render3d/perf/ab.sh` and `compare.sh` read `Render3D.log` / `Display8.log` there too; `a2film`'s UI-mark tail follows the log to wherever it appears.

Installed, not yet seen in game. (9a5e70a)

## 3.77.0 — 2026-10-30

### Added
- Scenario `display-claims`: three games on the stub, Display8 alone, with the UI, and
  with render3d at `[Modules] sky=0`, each logging who claims what;
  `display/bench-claims.sh` leaves plugins out of a clone by the player's name.

Installed, not yet seen in game. (cf3dec5)

## 3.76.0 — 2026-10-30

### Added
- Scenario `display-abi-mismatch`: two games, 13.42.4's plugins beside this checkout's
  `d3d8.dll` and the reverse, each standing down cleanly through passthrough.
  `display/bench-abi.sh` puts a commit's files into a clone (built once into
  `$A2_DATA/builds/abi/<commit>/`).

Installed, not yet seen in game. (c470071)

## 3.75.0 — 2026-10-30

### Added
- `render3d-pivot` (render3d's pivot marker in the tactical view, the engine drawing
  nothing there) and `render3d-pivot-stock` (`Pivot=0` on the stub: the engine's marker
  through the passthrough). Installed, not yet seen in game (1da0694)

## 3.74.2 — 2026-10-30

### Fixed
- `start_null_sink` leaked a same-named null sink per launch: a relaunch reuses the
  session's sink, teardown unloads every module of the session, and a launch sweeps
  the sinks of sessions that are gone (`session.py`). The cause of the main audio
  stream reaching the real output is not found; see the README trap.
- `no-assets` step 15 expects render3d's menu scene under the UI's column, not stock
  title art.

Installed, not yet seen in game. `6070901`

 — 2026-10-29

### Fixed
- The audio guard no longer stops the game for a corked stream on a real output:
  winepulse's start-up "format test stream", opened corked on every sink and closed
  within milliseconds, was the "game is no longer running" at launch (`_silence`). `8c0d697`

Confirmed in game 2026-10-30.

## 3.74.0 — 2026-10-29

### Added
- `ui-leftovers-2`: the pointed unit's damaged systems, the warp cursor over the
  minimap, the shot director's scenes in the tactical view and the frame rate
  (`FRAMERATE_DISPLAY=1` in the clone) drawn by the UI, with the engine drawing
  nothing in the mission (ui 0.25.0).

Confirmed in game 2026-10-30.

## 3.73.0 — 2026-10-29

### Added
- `a2test drive record NAME` (`bench/record.py`): one frame of the plugins' `gpu` calls
  on Display8's Vulkan backend, moved with the frame as presented (`game.ppm`) and the
  clone's settings into `$A2_DATA/replays/NAME/`, for `gpu/replay.sh` (`README.md`,
  "Recording a frame of gpu calls"). `239f29c`

Confirmed in game 2026-10-30.

## 3.72.0 — 2026-10-29

### Added
- `display-passthrough`: render3d's hulls module off on Display8's stub with
  `Passthrough=1`; the ships the engine draws must show, drawn into render3d's scene.
  `display-passthrough-forward` is the same scene through the forward chain, its
  reference (`baseline record`/`compare`). `24be351`

Confirmed in game 2026-10-30.

## 3.71.0 — 2026-10-29

### Added
- `display-inventory-modes`: phase 0 of the display plan's remaining contexts, with
  Display8 in forward mode and tracing: a mission in the showcase scene, a display mode
  chosen in Graphics Settings (the engine's `CreateDevice` at 800x600) and right-drag
  pans (`SetCursorPos` and its callers), each a marked context. Its stub variant
  (`display/stub-scenario.sh`) stops at the mode box: the stub offers one mode.

Confirmed in game 2026-10-30.

## 3.70.0 — 2026-10-29

### Added
- `--vulkan-device gpu|lavapipe` (`A2TEST_VULKAN_DEVICE`) and `--no-validate`
  (`A2TEST_VULKAN_VALIDATE=0`) on `run` and `session start`.

### Changed
- `Backend=vulkan` (and `auto`) bench games now run on the user's GPU by default, with
  the Khronos validation layer on, instead of lavapipe: `vulkan_env` no longer sets
  `VK_DRIVER_FILES`/`VK_ICD_FILENAMES` unless lavapipe is asked for, and
  `display/bench-mode.sh` sets `Device=llvmpipe` only then. `A2TEST_VULKAN_GPU=1` is
  now the default and ignored. The scenario notes that told to set it were updated.

Confirmed in game 2026-10-30.

## 3.69.1 — 2026-10-29

### Fixed
- `prepare` sets the clone's `ARMADA.PRF` interface mode to 1 (`set_prf_interface_mode`):
  the clone inherited the player's, and `ui-whole-game` and the `display-*` scenarios,
  which press Ctrl+I three times to reach mode 0 (the unit view), ended in a mode that
  hides the whole HUD when the player's was 0 (steps 54, 56, 61, 74, 75 failed).
- `ui-whole-game`: the loading-screen check waits 2 s after "O.K." instead of 6 s (the
  mission now loads in about 4 s, so the shot caught the fade into the mission); the
  objectives button is `hud-right 1555,55` (1502 is the view toggle since the button row
  gained its idle button).
- `render3d-mode-switch` runs on the forward chain (`Setup: display/bench-mode.sh
  forward ...`: the stub offers only the desktop's mode, so "800x600x32" is not in the
  list) and waits 10 s before the "after" shot (the log lines come at once, the first
  frame at the new size seconds later; 3.47.0's wait for logs had dropped the 10 s).

Confirmed in game 2026-10-30.

## 3.69.0 — 2026-10-25

### Added
- `display-movie-no-proxy`: the launch reels and the intro on Display8's stub without
  the cutscene proxy, as the release package installs them (ui 0.23.0). `6d7180d`

Confirmed in game 2026-10-30.

## 3.68.0 — 2026-10-23

### Added
- `render3d-nebula-explosion`: on Display8's Vulkan stub, a Sovereign blown up inside
  the Mutara gas and another just outside it, paused 22 and 60 ticks after each, for
  how far a fireball lights the gas.

Confirmed in game 2026-10-30.

## 3.67.0 — 2026-10-23

### Added
- `Show a bar along the top of the screen` / `Hide the bar`: a desktop panel's
  stand-in, `swaynag` on the overlay layer, that takes the pointer and not the keyboard
  (`logs/bar.log`; `Session.show_bar`, `hide_bar`).
- `Take the focus from the game with a screenshot`: as Omarchy's Print key, a frozen
  screen (`hyprpicker -r -z`) under the `slurp` (it was another name for the overlay).
- `Move the mouse into the bottom right corner`: puts the compositor's and the game's
  pointers back together after a change of focus parted them (`Session.to_corner`).
- `A2TEST_XWAYLAND_DEBUG=1`: Xwayland's Wayland protocol trace in `sway.log`.
- `ui-pointer-bar`: three such screenshots with a bar up and a walk into it after each,
  with `[UI] KeepClip=1`: the re-clip runs, no jump of the pointer.

Confirmed in game 2026-10-30.

## 3.66.0 — 2026-10-23

### Added
- `render3d-cone-emitter`: the Assimilator's auto-assimilator on Display8's stub with
  `Backend=auto` (Vulkan, on the GPU with `A2TEST_VULKAN_GPU=1`), shots of its emitter
  flare, which drew as a black square.

Confirmed in game 2026-10-30.

## 3.65.0 — 2026-10-23

### Added
- The step `Take the focus from the game [with a window | with an overlay] [for N
  seconds]`: a terminal on the session's sway takes the focus and is closed, or a
  region screenshot's overlay (`slurp`) has a region dragged on it (`logs/slurp.log`);
  `Session.take_focus`.
- `ui-focus-pointer`: a right-drag, each way of losing the focus, moves and a right-drag
  after, with `[UI] PointerLog=1`: no jump of the pointer, the engine not left holding
  it.

Confirmed in game 2026-10-30.

## 3.64.0 — 2026-10-23

### Added
- The `beam-pass` scene (a Galaxy class firing past a parked destroyer and an asteroid
  field at a Borg cube) and `render3d-beam-light`: paused frames for render3d's phaser
  beams as line lights.

Confirmed in game 2026-10-30.

## 3.63.0 — 2026-10-23

### Added
- `render3d-hull-sparks`: the shields-down discharges in the Mutara gas, and a Borg
  scout's damaged sensors in two frames, on Display8's Vulkan stub.

Confirmed in game 2026-10-30.

## 3.62.0 — 2026-10-23

### Added
- The `nebula-battle` scene (two Borg cubes and eight ships firing at them inside the
  Mutara nebula) and `render3d-nebula-lit`: a paused frame, frames in motion and a
  ship blown up in the gas, timed, for render3d's light inside the nebulae.

Confirmed in game 2026-10-30.

## 3.61.0 — 2026-10-23

### Fixed
- `ui-hud-bar` and `ui-hud-map` press Home (`center_player`) before they select, and
  click at the places measured after it (the first Assembler at 640,175, the Nexus at
  870,290 at 1280x720; `ui-hud-bar` places the station at 450,450): the Warzone start
  view opens zoomed out in some runs, where the click at 890,60 selected nothing.
  `ui-hud-bar` now checks each selection in `UI.log` ("| Assembler |", "| Nexus |").
- `ui-hud-map`'s minimap check asks for the base's race emblem and lets it cover the
  clearing, which it does when the start is in a corner of the map.
- `README.md`: parallel installs of one tree, and Home before a click on a unit.

Confirmed in game 2026-10-30.

## 3.60.0 — 2026-10-23

### Added
- `ui-placement`: a starbase placed in the `stations` scene, blocked against the yards
  and clear in open space, for the UI's placement markers (any Display8 mode through
  `display/stub-scenario.sh`).

### Changed
- `ui-hud-bar` waits on "placement: footprint" and judges the placement's new look.

Confirmed in game 2026-10-30.

## 3.58.0 — 2026-10-23

### Added
- The `battle` scene (sixteen ships and a planet, over a hundred lights) and
  `render3d-clustered`, which pauses it on one tick for render3d's clustered lights and
  their GPU timings (`render3d/bench-ini.sh`).

Confirmed in game 2026-10-30.

## 3.57.1 — 2026-10-23

### Changed
- `display-inventory` and `display-engine-draws` switch Display8 to forward mode
  themselves (`display/bench-mode.sh forward`), since the shipped default is now the
  stub; `display-stub` and `display-movie` describe the stub on `Backend=auto`.

Confirmed in game 2026-10-30.

## 3.57.0 — 2026-10-23

### Added
- `display-fallback`: Display8 in stub mode with `Backend=auto` and
  `FailBackend=vulkan,d3d12`; checks that Vulkan and Direct3D 12 are taken down and the
  engine's first `CreateDevice` lands on Direct3D 9, that the UI and render3d draw on it, the main menu
  and an Instant Action match.

### Changed
- `vulkan_env`: a stub on `Backend=auto` or a list that names Vulkan, with
  `Validate=1`, gets the Khronos validation layer (`VK_INSTANCE_LAYERS`); the drivers
  stay as they are (the GPU). Only `Backend=vulkan` alone is held to lavapipe.

Confirmed in game 2026-10-30.

## 3.56.0 — 2026-10-23

### Fixed
- `qol-refused-advisor` selects the starbase after Home (`center_player`), which puts it
  at 835,250 whatever the random start position; the start view's own place for it
  varies from run to run, and the old click at 735,305 selected nothing in some runs, so
  its presses queued nothing. It now checks each link: the selection (`info: Builder
  Starbase`, the bar's layout), the keys the bar took (`Q=build`, `Q=fconst`), the local
  bank's payments (3250, 3000), the refusal, the advisor for the local team and its chat line. Its log waits end on lines
  written after the action (the startup line is checked before LAUNCH).

Confirmed in game 2026-10-30.

## 3.55.0 — 2026-10-23

### Added
- `display-movie`: the launch reels and the intro on screen (a 4:3 picture between
  bars, not a black frame), through the UI; on the stub with `display/stub-scenario.sh`.
## 3.53.0 — 2026-10-23

### Added
- A game with `Mode=stub`, `Backend=d3d12` and `Validate=1` in its clone's
  `Display8.ini` gets the Khronos validation layer under vkd3d-proton, writing
  `logs/vvl.log` (`session.vulkan_env`).

Confirmed in game 2026-10-30.

## 3.52.0 — 2026-10-23

### Added
- The stock cache (`bench/stockcache.py`): a session that starts from stock (`--mod
  stock`, `--install`, `--assets none`) copies the game from `~/.cache/a2test/stock/<key>/`,
  where `a2mod stock` has run once per state of the install, instead of running it on
  every clone (12 s a session). Keyed by the install's tree (`lstat`, ctime included)
  and the `a2mod` script; built under a flock and renamed into place.
  `a2test stock-cache status | rebuild | drop`; `A2TEST_STOCK_CACHE=0` turns it off.

Confirmed in game 2026-10-30.

## 3.51.0 — 2026-10-23

### Added
- A game whose clone's `Display8.ini` has `Mode=stub` and `Backend=vulkan` is launched
  on lavapipe with the Khronos validation layer (`VK_DRIVER_FILES`,
  `VK_ICD_FILENAMES`, `VK_INSTANCE_LAYERS`; `session.vulkan_env`);
  `A2TEST_VULKAN_GPU=1` leaves the drivers as they are.

Confirmed in game 2026-10-30.

## 3.50.1 — 2026-10-23

### Fixed
- `ui-hud-map` and `ui-hud-bar` select the first Assembler where the start view puts it
  (890,60, at the top) and the Nexus at 1150,300: at 640,262 they clicked empty space,
  and every step after the selection failed, on `main` as on Display8, forward or stub.
  `ui-hud-map` puts the Assembler in group 1 and double-taps it to centre the camera
  for the bracket check; its minimap check names the SHIPS and STATIONS buttons (not
  "ALL") and the camera's outline as a few pixels at the start's zoom. `ui-hud-bar`'s
  checks name the minimap and the tooltip as they are drawn, and it expects the UI's
  build footprint ("leftovers: build footprint at").

Confirmed in game 2026-10-30.

## 3.50.0 — 2026-10-23

### Added
- Scenario `display-engine-draws` (`--install . --install display`): the intro movie,
  the first Federation mission's opening cinematic, the mission with a drag box, a
  pointed unit and move orders, the unit view and the pause, each expected to show no
  draw from `Armada2.exe`; and the in-mission menu over the UI's own copy of the frame.

Confirmed in game 2026-10-30.

## 3.49.0 — 2026-10-23

### Added
- Scenarios `port-showcase`, `port-nebula`, `port-firing` (one paused frame each with the
  bench clock, a shot and a frame dump: what a port of the renderer is compared by) and
  `display-stub` (the whole game with Display8 in stub mode).

Confirmed in game 2026-10-30.

## 3.48.0 — 2026-10-23

### Added
- Steps for Display8's tracer (`display/README.md`): `Mark the display context "NAME"`
  (writes `Display8.mark`, waits for d3d8.dll to take it) and `Expect the engine to draw
  nothing in "NAME"` (no `Draw*` from `Armada2.exe` in that context, from
  `Display8.inventory.tsv`). `Display8.log`, `Display8.ini` and the inventory are
  gathered with the logs.
- Scenario `display-inventory`: the whole game, a context marked at each stage, with
  `--install . --install display`.

Confirmed in game 2026-10-30.

## 3.47.0 — 2026-10-23

### Added
- `Wait up to N seconds for the logs the next steps expect`: a wait that ends as soon as
  every line the following `Expect "LOG" contains "TEXT"` steps check has been written
  since the last action, else after N s. 48 fixed waits of 5 s or more in 39 scenarios
  (1825 s at one resolution) now use it.

### Changed
- `Wait up to N seconds for "LOG" to say "TEXT"` counts only lines written since the last
  action (launch, click, key, typing, drag, Scene command). At the time limit, a line
  from before it passes and none is a failed check that lets the run go on (it was an
  error that ended the case). Polls every 0.5 s.
- A launch also deletes `UI.log` and `Online.log` (both plugins rewrite them as they load).
- `Quit the game` goes on as soon as the screen has the next button instead of waiting
  8 s for an exit after every press: from a mission it took ~19.5 s, two presses that
  could not end the game waited out.

Bench-only; installs nothing into the game.

## 3.46.1 — 2026-10-23

### Fixed
- `a2test watch`: a screen tile is plain black until its frame arrives, instead of a
  striped placeholder that flickered with black while a session or game was launching;
  the resolution is no longer drawn across it (the chip still shows it).

Confirmed in game 2026-10-30.

## 3.46.0 — 2026-10-22

### Added
- `a2test watch --install-desktop` / `--uninstall-desktop`: an app-launcher entry,
  `a2test-watch.desktop`, that runs the main checkout's `a2test watch --desktop`
  whichever worktree installs it. `--desktop` logs to `~/.cache/a2test/watch-desktop.log`,
  says a failure with `notify-send`, and focuses a running watcher's window instead of
  failing on its port.

Installed, not yet seen in game (tried on this desktop, Hyprland 0.56: launch, focus, close, failure notice).

## 3.45.0 — 2026-10-22

### Added
- `a2test watch` serves the bench dashboard (`bench/dash.html`, `bench/dash.py`) on the
  same port, 5950: a column per job (a run's cases, a film's players and an A/B's
  sessions are one column each), the queue with why each waiter waits, what finished in
  the last 2 h, CPU, memory, swap, GPU and VRAM gauges (amdgpu sysfs), and the judge's
  tokens today. State over Server-Sent Events (`/events`, `/api/state`); live noVNC for the
  maximized column only, from a `wayvnc` started on demand; in-memory `grim` snapshots
  (`/snap/<id>.jpg`, at most 5 s old) for the rest; nothing live while a perf A/B runs.
- noVNC 1.6.0 vendored in `testbench/vendor/novnc-1.6.0/`: the page loads nothing from the
  network.
- Who started a job, recorded at `a2test run`, `session start` and `a2film`
  (`bench/who.py`): the Claude Code session, main or subagent and the parent's worktree,
  or by hand; the worktree, branch and commit. Job records in `~/.cache/a2test/jobs/`
  (`bench/jobs.py`); sessions, slot sidecars and waiters carry the job's id
  (`A2TEST_JOB`); a session under `render3d/perf/ab.sh` belongs to the A/B.
- `a2test drive note "..."` records the driver's intent (`intent` in `steps.jsonl`), and
  `session start --note "..."` the first one. Every record of a drive command carries
  its command line and an invocation id (`src`, `cmd`, `inv`).
- The judge's token usage per call (`claude -p`'s `modelUsage`), in the step's `judge`
  note and summed per day in `~/.cache/a2test/judge-usage.json`.
- A scenario's `## ` headings name its phases. `slots.held_by_kernel()` reads the held
  slots from `/proc/locks` without taking a lock; a session's teardown notes `freed` in
  its slot's sidecar. `bench/dash_selftest.py`.

### Removed
- `bench/watch.html`, the tiled view, and its noVNC import from jsdelivr.

Installs nothing; seen working on the bench 2026-10-22.

## 3.44.0 — 2026-10-22

### Added
- `a2test drive dump [NAME]` and the scenario step `Dump the frame called "NAME"`:
  both plugins' frame dumps (Render3D's targets as PFM/PPM, UI's presented frame as
  PPM) asked for through their trigger files, moved into the session's `dumps/NAME/`
  with PNG previews and per-file min/mean/max (`bench/dumps.py`). A session gathers a
  left-over `Dumps/` into `logs/Dumps/` when it stops.

Installs nothing; seen working on the bench 2026-10-22 (`showcase`, 1280x720).

## 3.43.0 — 2026-10-22

### Added
- `./a2test baseline record | compare | diff | list | show` (`bench/baseline.py`): named
  frames in `$A2_DATA/baselines/<NAME>/` with their provenance (`meta.json`), and a pixel
  diff per shot: RMSE, max, pixels over `--threshold`, PSNR, a heat map and a side by
  side (`--gif`: a flicker GIF); `--rmse`, `--max-over` set the exit status. Reads PNG,
  PPM (P6/P5, 8 and 16 bit) and PFM, compared in linear float and after a fixed tone map.
- Scenario steps `Record the screenshot "NAME" as baseline "B"`, `Record every
  screenshot as baseline "B"` and `Expect the screenshot "NAME" matches baseline "B"
  [within RMSE X] [with at most N% of pixels over T]`.
- `bench/baseline_selftest.py`: every number, the readers and the exit codes on
  synthetic frames, no game.

Installs nothing; seen working on the bench 2026-10-22 (`showcase`, paused, 1280x720).

## 3.42.0 — 2026-10-22

### Added
- `--clock wall|bench` on `a2test session start` and `a2test run`, a scenario's `Clock:`
  header and `A2TEST_CLOCK`: `bench` sets `[Bench] Clock=bench` in the clone's
  `Render3D.ini` and `UI.ini`.
- `Scene.asi`: `Seed=` and `seed <n>` (the engine's three random sequences started again
  from a seed, as the mission loads and as the scene is built), `Step=` (every tick that
  many ms, whatever the frame rate), `Pause=` and `pause-at <tick>` (pause a fixed number
  of ticks after the scene is built), `tick`; `pause` answers its tick and the sim time.
  `SCENE_SEED`/`SCENE_STEP`/`SCENE_PAUSE` at install, and `Key=value` words after the
  scene in `bench-setup.sh`.
- Steps `Wait up to N seconds for "LOG" to say "TEXT"` and `Expect shots "A" and "B" to
  match within T`; scenario `bench-determinism` (ecbb58b, 4a64407).

Confirmed in game 2026-10-22.

## 3.41.2 — 2026-10-22

### Removed
- A session no longer copies the user's vkBasalt config or launches with
  `ENABLE_VKBASALT`, and gathers no `vkBasalt.log`: the vkBasalt bloom is gone (13.0.0).

Confirmed in game 2026-10-22.

## 3.41.1 — 2026-10-22

### Fixed
- `umu_run.py` is found beside `A2_PROTON` (Heroic's `tools/runtimes/umu/`), so the
  bench launches when Heroic is the Flatpak; `A2_UMU` still overrides.

Installs nothing.

## 3.41.0 — 2026-10-21

### Added
- Scene `fog-blackhole`: a black hole and a quantum singularity left under the fog of
  war, and the scenario `fog-blackhole` for render3d's ghosts of additive meshes
  (issue #3, 7e7a184).

Installs nothing; seen working on the bench 2026-10-21.

## 3.40.0 — 2026-10-21

### Added
- `Scene.asi`'s `cargo <name> <resource> [amount | full | empty]`: a freighter's hold,
  as mining fills it (`Freighter::LoadResource`).
- Scene `freighters`: the five freighters parked in a row, for the latinum trail and
  the cargo markings; scenario `render3d-freighter-trail` (7c49408).

Installs nothing.

## 3.39.0 — 2026-10-17

### Added
- `Scene.asi`'s `Captions=` key: 0 removes a mission script's captions (the intro
  cinematics' subtitles and stardate lower thirds, the mission's title) by sending
  `ScriptInterfaceImp::ShowTitle`'s and `ShowSubtitle`'s `OverlayText::AddMessage`
  calls to a stub that adds nothing. Works with `Enable=0`, in stock and refit. 1 (the
  default) is unchanged.

Installs nothing; seen working on the bench 2026-10-17 (`films/intros-scout` s01 refit
and stock, s02, s03 with `Enable=0`: every subtitle, lower third and title gone, the
cinematics' cuts and fades within 0.15 s of the earlier takes).

## 3.38.0 — 2026-10-17

### Added
- Scene `assimilation-fusion`: the technology assimilation beam from a Borg Fusion
  Cube (`bbattle3`) onto the Sovereign, 300 apart, for footage.
- Scene `assimilator`: the Borg Assimilator (`bcruise2`) and its auto-assimilator
  (`gborgbor`) on a healed Sovereign, 300 apart, for footage.

Installs nothing; both beams seen firing on the bench 2026-10-17 (stock).

## 3.37.0 — 2026-10-17

### Added
- `Scene.asi`'s `Race=` key (`federation`, `klingon`, `romulan`, `cardassian`, `borg`,
  `species8472`) and `SCENE_RACE=<race>` at `testbench/scene/install`: pins the
  player's race, and with it the HUD's skin (stock) or theme (refit, `Theme=auto`),
  which `-nointro a2_borg01` otherwise draws at random on every launch. It works with
  `Enable=0`. Empty (the default) leaves the game's choice.

Installs nothing; seen working on the bench 2026-10-17 (`showcase`, `Hud=1`, `Race=federation`:
the Federation HUD on 3 of 3 launches in stock and 3 of 3 in refit; `SCENE_RACE=klingon`
over it: the Klingon theme).

## 3.36.0 — 2026-10-17

### Added
- Scene command `tech <team | object> <item> [on|off]` and object key `Tech=`: a
  tech-tree item made available to a team as if researched, possible again where the
  map's tree forbids it.
- Scene `assimilation`: a Borg cube and an enemy Sovereign whose corbomite is
  available, so the technology assimilation beam holds (about 6 s).

### Fixed
- Scenario `specials`: the assimilation beam now forms (`tech tgt gcorbom` before it
  is fired); it ended on the tick it was fired, because `a2_borg01`'s tech tree makes
  the Sovereign's corbomite impossible.

## 3.35.1 — 2026-10-17

### Changed
- Scenario `ui-widgets-cursor`: Scene.asi's `cursor off` then `cursor on`, with
  `UI.asi`'s cursor hidden and shown with it (ui 0.12.1).

Confirmed in game 2026-10-30.

## 3.35.0 — 2026-10-17

### Added
- `Scene.asi`: `Zoom=` in `[Scene]` and the `zoom [<level>]` command set the RTS
  camera's zoom as a level from 0 (closest) to 1 (farthest), through the camera in use's
  `SetZoomLevel`, and answer with the level, the camera and its raw limits; `query` ends
  with the same line. `Zoom=0.37` is the view of 150 `ctrl+Down` then 35 `ctrl+Up`.
  Scene `zoom`.

Confirmed in game 2026-10-30.

## 3.34.0 — 2026-10-17

### Added
- Scenario `ui-hud-layout`: the UI's HUD at 1024x768, 1920x1080 and 2560x1080 with 1,
  6 and 30 ships selected, failing when panels overlap, when a panel at the bottom
  rises more than 45% up the screen, or when the panels cover more than 30% of it (60%
  and 42% on screens narrower than 3:2, the user's call for 4:3); from UI.asi's own
  rectangles (ui 0.12.0, `[HUD] LayoutLog=1`).
- Checks `Expect the HUD's panels not to overlap`, `... at the bottom to rise at most
  N% of the screen's height`, `... to cover at most N% of the screen`, each with an
  optional `(M% at 4:3)` for screens narrower than 3:2, and `Take a
  layout screenshot called "NAME"` (the shot with the panels drawn on it).
- Scenario header `Sheet:` (shot names): the run writes `<scenario>/sheet.png`, a row
  per case and a column per shot, and links it from the report.
- Scene `ui-hud-layout`: thirty Federation destroyers, `s1`..`s30`.

### Changed
- Scene.asi's `select` takes up to 39 names in one command (a command had 12 words).

Confirmed in game 2026-10-30.

## 3.33.0 — 2026-10-17

### Added
- `session start --mute CATS`: the game's `music`, `voice`, `effects` and `interface`
  volumes at 0 in the clone's `ARMADA.PRF` (its third line), the user's install untouched.
- `session start --stems CATS`: `music`, `voice` and `interface` each played on a null
  sink of its own (`a2test-<id>-<category>`) by `stems/Stems.asi`, which moves that
  category's 2D sounds to a DirectSound device on it; the rest stays on the session's
  sink as the effects stem. Stems are set to full volume. `Stems.log`, `Stems.ini`
  gathered with the logs (e682c70).

Installed, not yet seen in game (measured on the bench: `a2_fed01` intro, three stems).

## 3.32.0 — 2026-10-17

### Added
- Event marks: `Scene.asi` writes `Scene.marks`, timestamped events on the host's clock
  (`epoch` lines from `GetSystemTimeAsFileTime`): camera move and settle, glide and spin,
  selection (count and names), cloak and decloak steps, fire bursts per shooter and
  weapon ODF (beams included), explosions, crafts destroyed and removed, commands and
  specials. Read each frame from the camera hook; nothing new is patched.
- Scene-file section `[Marks]` (`Enable`, `FireGap`, `Still`) and the command `marks`.
- The bench clears `Scene.marks` at launch and gathers it with the logs (1aacd31).

Installed, not yet seen in game (seen on the bench 2026-10-17, `films/marks`).

## 3.31.0 — 2026-10-17

### Added
- Game slots (`bench/slots.py`): one machine-wide queue for game instances, `N` flock
  files under `$A2_SLOTS_DIR` (default `$XDG_RUNTIME_DIR/a2-slots`, else
  `$A2_DATA/locks`). `Session.create` takes a slot (or one handed down in `A2_SLOT_FD`)
  and the session's sway and launcher hold it until teardown; `a2test run` takes one per
  case, all of a `Players:` case's at once. `--jobs` above the count queues.
- `A2_SLOTS` (environment, or `armada2-refit.conf`) sets the count; the default is 2 to 3
  from cores and memory. `A2_SLOTS_DIR`, `A2_SLOTS_POLL`.
- `a2test slots`: holders (pid, worktree, label, session, since) and waiters.
- `bench/slots_selftest.py`: dummy-process checks of the queue.

Tested with dummy processes and three real session starts; not yet seen under load.

## 3.30.1 — 2026-10-17

### Changed
- `ui-hud-info` and `ui-hud-bar` click where ui 0.11.1's bottom row puts the queue,
  the group's tiles and the bar.

Installs nothing.

## 3.30.0 — 2026-10-17

### Added
- Scene `fog-planet` and scenario `render3d-planet-ghost`: a class M planet left as a
  fog-of-war ghost, which keeps render3d's sphere and surface.

Confirmed in game 2026-10-17.

## 3.29.0 — 2026-10-17

### Added
- Scenario `ui-hud-minimap-warp`: three destroyers selected, the cursor over the
  minimap is the warp cursor (ui 0.11.0).

## 3.28.0 — 2026-10-17

### Added
- Scenario `ui-hud-polish`: *Lifelines* through the campaign menu at 1680x720, the
  UI's trade status, the starbase's shorter info panel, the bar's icons and the
  minimap's filters, and the AI menu's level icons (ui 0.10.0).

Confirmed in game 2026-10-17.

## 3.27.0 — 2026-10-17

### Added
- Scenario `ui-loading-warp`: a campaign mission started from the campaign selection
  departs from the menu scene into the warp tunnel under the loading screen (ui 0.9.0,
  render3d 0.10.0).
- Scenario `ui-loading-pace`: a campaign load with no hold, for UI.log's slow steps and
  longest wait. Both click the UI's mission list by its labels.

### Changed
- `Wait for the briefing` presses Space at every look until the briefing shows, which
  skips a campaign mission's opening cinematic (`ui.json` screen key `skip`).

Confirmed in game 2026-10-17.

## 3.26.0 — 2026-10-16

### Changed
- Scenario `ui-menus-scene` visits each campaign's scene by hovering its row (ui 0.8.0,
  render3d 0.9.0); `ui-menus-single`'s campaign check accepts the column of emblems.

### Added
- Scenario `ui-menus-scene-abort`: the menu scene is back on the campaign selection
  after a mission is aborted (ui 0.8.0).

Confirmed in game 2026-10-16.

## 3.25.0 — 2026-10-16

### Added
- `ui.json` targets take a `label`: `Click the <target>` clicks it by OCR while UI.log
  says the UI's menus stand over render3d's scene (`menus: scene <name> on`), and the
  bench's quit does the same for Exit; the main menu's and the campaign selection's
  targets have one.
- Scenario `ui-menus-scene` (ui 0.7.0, render3d 0.8.0).

### Changed
- `ui-menus-single`'s campaign check accepts the cards in a row over the scene.

Confirmed in game 2026-10-16.

## 3.24.0 — 2026-10-16

### Added
- Scene `ui-colonize` and scenario `ui-hud-colonize`: a planet selected by a click and
  colonised by a right click (render3d 0.7.1).

Confirmed in game 2026-10-17.

## 3.23.0 — 2026-10-16

### Added
- Scenarios `ui-whole-game` (16:9, 21:9 and 4:3: the whole game with `./install`'s
  defaults), `ui-menus-multiplayer`, `ui-hud-multiplayer` (ui 0.4.0); operating notes
  `ui-switchon.md`, `ui-multiplayer.md`, `ui-fixes.md`.

### Changed
- Scenarios of the superseded plugins (`grid-layout`, `hud`, `hud-mode-switch`,
  `escape-menu`, `main-menu`, `game-setup-labels`) put their plugin back with
  `ui/bench-legacy.sh`; `_enter-instant-action` goes through the UI's Game Setup;
  `ui-hud-map`, `ui-hud-panels` and `ui-hud-panels-objectives` check what stock shows.
- A run's report gathers more of the UI's logs.

Confirmed in game 2026-10-17.

## 3.22.0 — 2026-10-16

### Added
- `Scene.asi` commands `succeed [s]` and `fail [s]`, which end the mission as its
  script would; scenes `ending`, `ui-hud-info`.
- Scenarios `ui-hud-info`, `ui-hud-map`, `ui-hud-overlay`, `ui-hud-panels`,
  `ui-hud-panels-objectives`, `ui-menus-single`, `ui-menus-dialogs`, `ui-menus-widgets`,
  `ui-menus-setup`, `ui-menus-success`, `ui-loading` (ui 0.3.0); operating notes for
  each agent's area.

### Changed
- `ui-menus-options` and `ui-menus-match` follow the menus' own layouts.

Confirmed in game 2026-10-17.

## 3.21.0 — 2026-10-16

### Added
- Scenario step `Scroll the wheel down|up [N times]` (481b796).
- Scenarios `ui-widgets`, `ui-widgets-cursor` (scene `ui-cursor`), `ui-style`,
  `ui-hud-bar`, `ui-menus-options` and `ui-menus-match` (ui 0.2.0).
- A run's report gathers `UI.log` and `UI.ini`; `design_to_screen` also reads the
  design-area line `UI.log` writes while the UI hosts the menus.
- Operating notes `operating/ui-widgets.md`, `ui-style.md`, `hud-takeover.md`,
  `menus-host.md`.

Confirmed in game 2026-10-17.

## 3.20.0 — 2026-10-16

### Added
- Scenario `ui-runtime`: `UI.asi`'s demonstration panel in a screen of ours (the
  Credits' place), on the loading screen and over the HUD, with a click on it that the
  engine must not see (ui 0.1.0). Its setup is `ui/bench-asi.sh`.

Confirmed in game 2026-10-17.

## 3.19.1 — 2026-10-16

### Changed
- `specials`: the holding, assimilation (`gTechAssimBeam`) and tractor beams are
  checked as `Kind=cone` and shot close and from above; `shields-hits`: a Klingon
  cruiser takes the same fire, for the rays.

Installs nothing; both scenarios pass on the bench.

## 3.19.0 — 2026-10-16

### Added
- Scene `sky` (`SCENE=sky`): no objects, the free camera at the middle of the map
  looking out, for judging the skies on their own (712a09a).

Confirmed in game 2026-10-17.

## 3.18.0 — 2026-10-16

### Added
- `a2test session start --assets none`: the clone is installed from an empty `A2_DATA`
  (no textures, menu plates or movies), as a scenario's `Assets: none` already could;
  the game as a release installs it. Used by `films/promo`.

Installs nothing into the game. Confirmed in game 2026-10-16.

## 3.17.0 — 2026-10-15

### Added
- Scenario `render3d-match`: an Instant Action match on render3d with every module on,
  a battle staged beside the Borg start by `Scene.asi`'s new `SCENE=match` (builds and
  hides nothing; fog off).
- Scenario `render3d-mode-switch`: a display mode changed in Graphics Settings during
  the `showcase` battle; render3d must take the new device and draw the same scene.
- Scenario `render3d-specialfx`: the special-object effects (Computer Override, Energy
  Dissipator, Psionic Disruption) through render3d's `specials` module.
- Scenario `render3d-ordmodels`: mines, the commando team, the myotron, the flux wave,
  transwarp and the chain pulsar fired in the `specials` scene, drawn by render3d.
  Scenes `mining` (a freighter mining a dilithium moon) and `ordmodels`.
- Scenario `render3d-hulls-extras`: the build ghost and height posts while placing a
  station, a hologram (a Romulan holo-emitter on a Klingon battleship) and the Evolver,
  drawn by render3d.
- Scenario `render3d-construction` and scene `construction`: a construction ship
  building a station, drawn by render3d. `Scene.asi` command `build <rig> <odf> <x y z
  | object>` orders a construction ship to build there (resources topped up).
- README "Operating the game: what agents learned" (one game at a time, 720p, timings,
  A/B at the same moment, scenario pacing), and `testbench/operating/`, one file of
  staging recipes per topic. `a2test run -j 1` named for one game at a time.

### Changed
- Scenario `no-assets`: `Planets.asi` is no longer installed beside render3d; the world
  module's load is checked in its place.
- Scenario `no-assets`: with render3d's modules in place of Sky, Explosions, Shields,
  Repair, Fog, Specials and Venting, it checks that `Render3D.asi` loaded each module.

### Changed
- Scenarios `explosions`, `shields-hits`, `shields-down-repair`, `venting`, `specials`,
  `fog-edges` and `cloak` check render3d's modules in `Render3D.log` in place of the
  removed plugins; the session no longer clears or collects their `.ini` and `.log`.

Confirmed in game 2026-10-17.

## 3.16.0 — 2026-10-15

### Added
- Scenario `render3d`: the `showcase` scene through `Render3D.asi`, the main view and,
  with `hud on`, the action camera's inset, checked in `Render3D.log`.
- `Render3D.log` and `Render3D.ini` are cleared at launch and gathered with the other
  logs.

Confirmed in game 2026-10-17.

## 3.15.0 — 2026-10-13

### Added
- `Vent=` (scene file) and the scene command `vent <name> <system|all> on|off`: a
  system's damage node shows as if damaged (the plume, the venting gas) on a healed
  craft, the system still working.
- `SCENE=venting`: a Galaxy class and a Klingon battleship parked with every damage node
  showing, for `Venting.asi`.
- Scenario `venting`: the plugin's hooks, both kinds of vent recognised, drawn and lit,
  shots parked, from below, moving and far. `no-assets` also expects `Venting.log`'s hooks
  line. The bench clears `Venting.log` at launch and gathers it with `Venting.ini`.

Installs nothing; the scene and the command seen working on the bench 2026-10-13.

## 3.14.0 — 2026-10-13

### Added
- `SCENE=cloak`: the player's own warbird and Bird of Prey in front of a planet and a
  Galaxy class, for cloaked ships drawn as glass.
- Scene command `cloak <name> on|off|now`, and each craft's cloak state in `query`.
- Scenario `cloak`: cloaks, holds and decloaks the warbird, checks the glass path in
  `Lighting.log`, and shoots each stage.

Installs nothing; seen working on the bench 2026-10-13.

## 3.13.0 — 2026-10-13

### Added
- `Scene.asi`: `special <ship> <weapon> [<target>]` fires a ship's special weapon (its
  weapon ODF's name): the weapon's slot in the ship's carrier, then
  `Craft::FireAtSpecial`, as a player's order does; it answers with the slot, whether the
  weapon can fire and whether it fired. `energy <ship> on|off` and the scene key
  `Energy=` keep a ship's special energy full every tick. Scenes may hold 96 objects
  (was 32).
- `SCENE=specials`: an enemy Sovereign that cannot die and does not fire, and a Galaxy
  off to one side to warp, for `Specials.asi`.
- Scenario `specials`: the holding beam, the tractor beam and the warp flash drawn by
  `Specials.asi`, and psionic insanity's wave shot, with shots. `no-assets` also expects
  `Specials.log`'s hooks line.
- The bench clears `Specials.log` at launch and gathers it with `Specials.ini`.

Installs nothing.

## 3.12.0 — 2026-10-13

### Added
- `SCENE=asteroids`: an asteroid field (`mafield`) and a big one (`mafield2`) beside a
  Galaxy class, for the asteroids' surfaces and smoothing.

Installs nothing; seen working on the bench 2026-10-13.

## 3.11.0 — 2026-10-12

### Added
- `SCENE=fog`: three of the player's ships with the map's fog and shroud on and the HUD
  on, for `Fog.asi`.
- Scenario `fog-edges`: the plugin's hooks, both layers drawn through the shader, a
  near and a far shot. `no-assets` also expects `Fog.log`'s hooks line.
- The bench clears `Fog.log` at launch and gathers it with `Fog.ini`.

Installs nothing; seen working on the bench 2026-10-12 (`fog-edges` passes; `no-assets`
passes its `Fog.log` step).

## 3.10.0 — 2026-10-12

### Added
- Scenario `shields-down-repair` (`SCENE=shields`): the target's shields shot out, the
  shields-down arcs and their light drawn by `Shields.asi`, then the attackers stopped
  and the repair welding and its light drawn by `Repair.asi`; shots close, from below,
  from RTS distance and of the welds. `no-assets` also expects `Repair.log`'s hooks line.
- The bench clears `Repair.log` at launch and gathers it with `Repair.ini`.

### Fixed
- `no-assets` and `shields-hits` look for the Shields hooks line by its weapon sites:
  with another plugin loaded first, `RenderChunks` is logged as chained and the old text
  never matched.

Installs nothing.

## 3.9.0 — 2026-10-12

### Added
- `SCENE=shields`: a Galaxy (phasers) and a Klingon battleship (torpedoes) firing at a
  second, healed Galaxy, for `Shields.asi`.
- Scenario `shields-hits`: the plugin's hooks, beam and missile hits drawn, three shots.
  `no-assets` also expects `Shields.log`'s hooks line.
- The bench clears `Shields.log` at launch and gathers it with `Shields.ini`.

Installs nothing; seen working on the bench 2026-10-12 (both scenarios pass).

## 3.8.0 — 2026-10-12

### Added
- `Scene.asi`: `kill <name>` makes a scene craft die at once
  (`ScriptInterfaceImp::ExplodeNow`), healing and immortality off first.
- The `explosion` scene: a Galaxy class beside a destroyer, a cruiser and a battleship
  to blow up; the `explosions` scenario (`explosions/README.md`), and an
  `Explosions.log` check in `no-assets`.
- The bench gathers `Explosions.log` and `Explosions.ini`.

Installs nothing; seen working on the bench 2026-10-12 (`explosions` and `no-assets`
pass).

## 3.7.0 — 2026-10-12

### Added
- The `impacts` scene: Klingon torpedoes and Species 8472 pulses striking two Galaxy
  class targets with no shield flash, for the lighting layer's `Impacts=`.

Installs nothing; seen working on the bench 2026-10-12.

## 3.6.0 — 2026-10-12

### Added
- Scene `planets`: one planet of each class and a dilithium moon, for `Surfaces.asi`.

Confirmed in game 2026-10-30.

## 3.5.0 — 2026-10-07

### Added
- `Scene.asi`: `Music=` (default 1, as stock): 0 stops the game starting any music track
  (`StartMusic` and `JukeBox::mStartNewTrack` return at once), leaving effects and voices;
  `SCENE_MUSIC=0` at `testbench/scene/install` puts it into the installed `Scene.ini`.
  It works with `Enable=0`, so a campaign take has no music and no scene.
  Footage is recorded without the game's music and laid over its own soundtrack.

Installs nothing; seen working on the bench 2026-10-07 (planet scene: -44 dB with music,
-60 dB without).

## 3.4.0 — 2026-10-07

### Added
- `Scene.asi`: `colonize <planet> [<population> | full | off] [team]` and the scene key
  `Population=` (with `Colonist=`) make a planet a grown colony without a colony ship;
  the `colony` scene (263e543).
- `Scene.asi`: `tooltips on|off` and `Tooltips=` (default off) hide the rollover boxes
  over the 3D view (634d3a9).
- `Scene.asi`: `goto <name>[,<name>...] <object | x y z> [warp]` and `stop <name>`, a
  player's move and stop orders (634d3a9).
- `Scene.asi`: `hover on|off` and `Hover=` (default on, as stock) remove the gauges,
  system icons and ring the object under the pointer gets.
- `Scene.asi`: `shieldfx on|off` and `ShieldFx=` (default on, as stock) remove the flash
  of weapon hits on shields; damage is unchanged.

### Changed
- `Scene.asi`: `Heal=1` keeps shields and crew full as well as the hull, so a healed
  craft never shows the shields-down ring (634d3a9).
- `Scene.asi`: `Heal=1` also repairs every system (`Craft::RepairAllSystemsComplete`),
  and no scene craft shows a damaged-system effect for a system the scene switched off
  (`Engines=0`, `Weapons=0`, `engines`), or for any system once healed: the plasma
  plume every `Engines=0` ship streamed is gone.
- The `showcase` scene: the cubes hold fire, so no Borg transporter effect in footage
  (cf4d277).

Installs nothing; seen working on the bench 2026-10-07.

## 3.3.0 — 2026-10-07

### Added
- `Scene.asi`: `glide <seconds> <yaw> <pitch> <distance>` eases the orbit camera to a new
  position, and `spin <degrees per second>` turns it steadily, both on wall-clock time
  per frame, for camera moves that record smoothly.
- The `showcase` scene: a Federation and Klingon fleet against two Borg cubes in front of
  a class M planet, for footage.

Installs nothing; seen working on the bench 2026-10-07.

## 3.2.0 — 2026-10-07

### Added
- The `warp` scene (three Federation destroyers in open space) and
  `scenarios/qol-warp.md`: a long move on the map goes to warp (qol 1.3.0).

Installs nothing; the scene seen working on the bench 2026-10-07.

## 3.1.0 — 2026-10-07

### Added
- The `factions` scene: one torpedo or pulse ship of each playable faction, each firing
  at its own Borg cube (lighting 1.12.0, `OrdnanceColours`).

Installs nothing into the game; seen working on the bench 2026-10-07.

## 3.0.0 — 2026-10-06

### Changed
- `a2test session stop` requires the session's ID (`session stop ID`, `--session ID` or
  `A2TEST_SESSION`); it no longer stops the newest session by default.
- `a2test drive` takes `--session ID` anywhere on its line. Without one (or
  `A2TEST_SESSION`) it drives the only active session and refuses when several are
  active, instead of driving the newest.
- `A2TEST_SESSION` takes a session ID as well as a state path; an unknown or ended
  session is an error rather than a fall-back.
- `session start` prints the `drive --session` and `session stop` commands for its ID.

Installs nothing; seen working on the bench 2026-10-06.

## 2.14.0 — 2026-10-05

### Added
- Scene.asi: `select <name>...` (as a click and Shift-clicks) and `selection` (what is
  selected, each one's group, queue and class, and every control group); `query` gives
  each producer's queue. The `stations` scene: three shipyards, an advanced one, a
  research station and two ships.
- Scenario steps `Scene "CMD"` and `Expect scene "CMD" answers "TEXT"`, and
  `testbench/scene/bench-setup.sh <scene>` for a scenario's `Setup:` line.
  `drive scene` and the steps share `Session.scene`.
- `scenarios/qol-station-groups.md`: stations in control groups and the build menu
  for several (qol 1.2.0).

Installs nothing into the game; seen working on the bench 2026-10-05.

## 2.13.0 — 2026-10-05

### Added
- `testbench/d3d9probe/`: `D3D9Probe.asi`, for bench clones only. It asks the game's
  device for Direct3D 9 and, when it gets one, draws through it: a shader model 3.0
  pattern every frame, and a `ps_2_0` swapped into the indexed draws while
  `D3D9Probe.tint` exists. `D3D9PROBE_D3D8=<dll>` puts a d3d8.dll into the clone's slot.
  `D3D9Probe.log` is gathered with the other logs. Its findings are in `platform/D3D9.md`.
  Its shaders are HLSL (`probe.hlsl`), built with `platform/d3d9/hlsl.sh` and used
  through `platform/d3d9/d3d9dev.h`, so it checks that piece too. It logs the first
  `CopyRects` of each pair of surfaces with the branch d3d8to9 takes, and
  `D3D9PROBE_TIMING=1` turns vsync off in the clone and the overlay off, so frame
  times compare across chains.

Installs nothing into the game; seen working on the bench 2026-10-05.

## 2.12.2 — 2026-10-04

### Fixed
- The audio watchdog's `pactl subscribe` dies with the process that started it. A
  session stopped by terminating its driver used to leave it running; 63 had piled up
  by evening, until pipewire-pulse refused every new client and no session could start
  ("could not create the silent sink"). (7360cb0)

## 2.12.1 — 2026-10-04

### Fixed
- A `session start` that failed part-way (on 2026-10-04, the audio guard stopping a
  launch whose stream reached a real output) left its sway, input and VNC running and
  the session listed as active with no game. It is now torn down and reported like any
  stopped session, and the error is raised as before.
- Sessions start in about a minute instead of three and a half: `./install` spent ~175 s
  in `a2tex install` (textures 2.1.1).

## 2.12.0 — 2026-10-04

### Added
- `testbench/scene/`: `Scene.asi`, a scene builder that goes into bench clones only
  (`--install testbench/scene`). Once a directly launched mission is running it turns
  fog and shroud off for good, hides the HUD, grid, cursor and event notices
  (`Fog=`, `Hud=`, `Grid=`, `Cursor=`, `Notices=`), and builds the objects of the scene
  file's `[Object.<name>]` sections (`Odf=`, `Team=`, `X`/`Y`/`Z`, `Heading=`,
  `Immortal=`, `Heal=`, `Engines=`, `Weapons=`, `Attack=`). `Anchor=`, `Center=`,
  `Camera=`, `Delay=` in `[Scene]`.
- Three scenes in `scene/scenes/`, chosen with `SCENE=`: `planet` (a Galaxy class beside
  a class M planet), `nebula` (beside the Mutara nebula) and `firing` (a Galaxy class
  firing at an indestructible Borg cube without moving). `SCENE_INI=` takes any file.
- `a2test drive scene "CMD" ...`: commands to a running scene through `Scene.cmd`. They
  include a free camera (`orbit`, `camera`, `camera rts`), `spawn`, `attack`, `heal`,
  `engines`, `weapons`, `immortal`, `center`, `pause`/`resume`, `hud`/`grid`/`cursor`/
  `notices`, and `query`. The bench gathers `Scene.log` and `Scene.ini`.

### Changed
- `a2test session stop` terminates the game instead of quitting through its menus;
  `--graceful` quits through the menus as before.

Installs nothing into the game; seen working on the bench 2026-10-04.

## 2.11.0 — 2026-10-04

### Added
- `testbench/d3dtrace/`: `D3DTrace.asi`, a Direct3D 8 call tracer that goes into bench
  clones only (`--install testbench/d3dtrace`), and `summarize.py`, which groups a
  traced frame's draws by lighting path. While tracing it also logs every
  `SetTransform` as a `T` line. A session's `D3DTrace.log` is gathered with the other
  logs, and so are `Lighting.log` and `Lighting.ini`.

Installs nothing; no game-side change to confirm.

## 2.10.0 — 2026-10-04

### Added
- `scenarios/_enter-instant-action.md`: from launch into an Instant Action match on
  Warzone as the Borg against one easy AI, in design coordinates. The way into any
  match with units; a direct map launch has none.
- `scenarios/grid-layout.md`: `GridLayout.asi`'s grid, keys, placement beside the info
  panel, the stock keys switched off, and chat left alone.
- `GridLayout.log` and `GridLayout.ini` are collected with the other plugins' logs.
- `GAMEPLAY.md`: getting a match with units, where the Borg start is on Warzone, the
  stock bar keys and the grid, cancelling a placement, and how a hung game looks.

Installs nothing; no game-side change to confirm.

## 2.9.0 — 2026-10-03

### Added
- Scenario step `Right-drag from X,Y to X,Y`: the drag step with the right button held,
  which pans the map in a mission.
- `QOL.log` and `QOL.ini` are collected with the other plugins' logs, and
  `scenarios/no-assets.md` checks that `QOL.asi` scaled the right-drag pan speed.

Installs nothing; no game-side change to confirm.

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
