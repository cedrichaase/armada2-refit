# Operating the bench: construction, worker bees, sparks (agent "construction")

Tested 2026-10-15, 1920x1080 (`--res 16:9`), `a2_borg01`.

## Starting
    SCENE=construction ./a2test session start --res 16:9 --install <copy> \
        --install <copy>/testbench/scene --args '-nointro a2_borg01'
- The scene is built ~30-60 s after start. Wait for `scene ready` at the end of
  `~/.cache/a2test/sessions/<ID>/game/Scene.log` (nothing else in that log says "built").
- The harness refuses `sleep N; cmd` in one shell call; poll instead:
  `until grep -q "scene ready" .../Scene.log; do sleep 3; done`.
- Shell calls that write files outside the worktree with heredocs/`python3 -` were
  sometimes refused by the sandbox ("too complex to verify"); the Write/Edit tools work.

- The bench's GPU is the user's: a GPU hang in a bench game (18:24 on 2026-10-15) took
  every bench game and the user's desktop compositor down. Run one game at a time,
  stop a session as soon as you have the shots, keep new shader passes fixed-cost.
  A session whose game died shows "game not running" in `./a2test session list`;
  `session stop` still cleans it up (exit code 2 is normal then).

## Getting a station built
- `scene "build rig fyard 2300 0 2000"` (scene command added for this work): the
  construction ship `rig` (fconst) flies there, opens, six worker bees (orange `fbee`)
  fly out, weld with `weldbeam` ribbons, and the yard rises face by face. The command
  tops the team's resources up. `query` shows the rig moving.
- The station is built at the point given (Render3D.log: `effects: construction: fbase
  at 2300,0,2000`); it is not a scene object, so look with `orbit 2300 0 2000 160 30 280`.
  Timing (from the scenario runs): the station starts within seconds of `build`;
  `fbase` (buildTime 100) is complete after ~100 s, so 50 s in it is half built (pause
  there to look); `fyard` (buildTime 25) is done in under 45 s. My Render3D.log counter
  ("every 600 draws") is per view drawn, not per second: don't read time off it.
- Cancel a construction: `scene "kill rig"` mid-build: the rig explodes and the
  part-built station breaks into chunks (Render3D.log `debris: fbase broke into N
  chunks`).
- Wait on log lines, not time: `until grep -qE "fbase at .*: [0-9]{4} of" the clone's prefix, `.../AppData/Local/armada2-refit/logs/Render3D.log`.
- `orbit <x y z> yaw pitch dist` takes world coordinates with `Anchor=world`.
- Don't use `drive step "wait 12"`: `step` runs an AI agent (costs money, slow). Poll a
  log or just take the next shot.

## Scenarios from a copy of the repo
- `Setup: testbench/scene/bench-setup.sh <scene>` is resolved against the repo `a2test`
  runs from (the real worktree), not the scenario's copy: a new scene or scene command
  in a copy is not there (`setup ... failed (exit 1)`, run 20261015-190441). Run a
  temporary copy of the scenario whose `Setup:` is the absolute path of the copy's
  `bench-setup.sh` (an absolute path is used as it is).
- A scenario with a judged `Check that` step runs a reference case at 4:3 (1600x1200)
  too, even with `--res 1280x720`; `--jobs 1` keeps it to one game at a time.

## A/B against stock at the same moment
- `[Construction] ABTest=1` in the clone's Render3D.ini (read at start; I put it in my
  copy's render3d/Render3D.ini for the bench), then `scene pause`, `shot ours`,
  `touch ~/.cache/a2test/sessions/<ID>/game/Render3D.construction-off`, wait for
  `the engine's (A/B)` in Render3D.log, `shot stock`; `rm` the file to switch back.
  Rendering continues while paused, so both shots are the same sim moment.

## Sparks (hits, system explosions)
- `SCENE=impacts`, then `scene "heal tgt off"`: the target's shields go down, phasers
  hit the hull. System explosions (`systemexpl*`) came on `tgt2` (the 8472 ship's
  target) after `heal tgt2 off` + `attack sp tgt2`; `orbit tgt2 200 10 120` and a burst
  of 4-6 shots catches some. A paused A/B often catches only old (smoky) sprites.
- `pflare` particles are the phasers' muzzle flares on the *firing* ship (kli), not hits.
