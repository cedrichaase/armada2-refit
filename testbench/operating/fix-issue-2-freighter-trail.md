# Operating the bench: loaded freighters, the latinum trail, the cargo markings

Tested 2026-10-21 on `a2_borg01` with `SCENE=freighters`, `--install <checkout>
--install <checkout>/testbench/scene`, at 1280x720.

## Loading a freighter

- A scene's freighter never mines (`special-weapons-and-mining.md`), so its hold is
  filled with `scene "cargo <name> <resource> [amount | full | empty]"`
  (`Freighter::LoadResource`). A freighter takes dilithium (150) and latinum (75);
  metal and biomatter clamp to 0 on every stock freighter. The answer says what is in
  the hold (`ok: latinum 75.0`).
- A full hold slows the ship to 0.4 of its speed (`speedFractionCarryingLatinum`), so
  under way it covers ~100 units in 5 s: frame the moving shot at 200-250.
- Keep `Engines=0` to park a loaded freighter, and `engines <name> on` before a `goto`.

## Stock against refit at the same moment

- The A/B for this needs two switches together, both read again while the game runs:
  `[Hulls] Reload=1` and `ABTest=1` in the clone's `Render3D.ini` with the file
  `Render3D.hulls-off` in the game directory (the hulls module stands aside), and
  `[Particles] Enable=0` (its `Reload=1` is the default). Neither section is in the
  shipped `Render3D.ini`: append them. Wait for `hulls: off (A/B)` and
  `particles: Enable=0 (Render3D.ini read again)` in `Render3D.log` before the shot.
- **Edit the right `Enable=`.** `[Render3D]` and `[Specials]` have one too: a
  `sed s/Enable=1/Enable=0/` over the file switched `[Specials]` off. Edit the line by
  number.
- **Particles cannot be compared while paused.** With the hulls module on, the engine
  never renders a freighter's instance and so never simulates its emitters: switched to
  the engine while paused, the trail is empty, and it only grows once the game runs.
  Compare the cargo markings paused, the trail unpaused (a few seconds each way).
- `engines`, `goto`, `cargo` all work while paused; the hold's markings follow at once.

## Driving

- A small script that takes the session ID and a list of `scene` commands, `sleep N`
  and `shot NAME` keeps each call one line (the sandbox refuses loops of `drive` in a
  command).
- Sessions start in about 2 minutes here with other agents' games on the slots; wait
  for `scene ready` in the clone's `Scene.log`.
