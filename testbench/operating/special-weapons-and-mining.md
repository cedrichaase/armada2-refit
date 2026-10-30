# Operating the bench: ordnance models, mines, mining beams (agent ordmodels)

Tested 2026-10-15 on `a2_borg01`, `--install <copy> --install <copy>/testbench/scene`.

## Mining beams

- A freighter the scene spawns **does not go mining**, and `ScriptInterfaceImp::Mine`
  (0x452b40; tried as a scene command, then dropped) does nothing either: a moon built by
  `BuildObject` (`mdmoon`) has no resource interface (`+0x17c`), which
  `Freighter::mCouldMine` wants, so the order is dropped silently. Don't spend time on it.
- What works: fire the mining weapon as a special weapon. `ffreight`'s `weapon1` is
  `gmineDilithium`, `weapon2` `gmineLatinum`; `bfreight` has `gmineDilithiumBorg` and
  `gmineMetalBorg`; the orbital stations fire `gOrbitalBeam`, the repair ships
  `gRepairBeam`:

      ./a2test drive --session ID scene "special freighter gmineDilithium moon"
      # -> slot 0, can fire, fired

  Fired at a ship it answers `cannot fire now`. The beam lives 1.25 s
  (`minedilithium.odf` `lifeSpan`): fire, then `scene pause` at once, then `shot NAME`.
  Its `range` is 180: a freighter 250 from the moon's centre answers `cannot fire now,
  NOT fired` (and an `answers "fired"` check passes on that: check `", fired"`).
- `SCENE=mining`: moon at 3000,0,3000, freighters in front. Close camera on the beam:
  `scene "camera 3120 30 2820 3060 0 2960"`.
- Stock looks: dilithium = two thin blue-white parallel lines (wmbeam.sod, Wtractor
  texture tinted by its material); latinum = a cone of green rays. The beam's end
  (+0xc8) is inside the moon, not on its surface.

## Special weapons that draw a model

- Ships: `fdestroy` gamine (antimatter mines), `kdestroy` ggrvmine (gravity mine),
  `kcruise2` gkcteam (commando team), `rcruise2` gmyotron, `rsuper` gbholeg (black
  hole), `ccruise2` gfluxwave, `bdestroy` gtwarpd (transwarp: the ship leaves),
  `fcruise1` gtempor and gchainp (chain pulsar). `SCENE=ordmodels` has most of them.
- `special <ship> gamine` with no target answers `cannot fire now, fired` and does
  lay a mine: **at the layer's own position, inside its hull**, where it stays. Get the
  layer moving first (`goto fd 3000 0 4900`, wait 3 s), then lay; the mines then sit
  behind it. Within ~600 of an enemy (`detectRadius`) they chase it and blow up, so
  lay them away from `tgt` if you want to look at them.
- Targeted ones (`gkcteam tgt`, `gmyotron tgt`, `gchainp tgt`) answer `can fire, fired`.
- Shooters of team 1 also fire their own weapons at `tgt` (team 2), which clutters the
  picture; it doesn't matter for logs.
- There is no scene command that lists ordnance. The ordnance module logs each ODF the
  first time it is submitted (`ordnance: <odf> (vtable, Submit): <who>`), and models
  log `drawing the model of <odf> (...) at x y z` the first time and every 600th
  draw: use that position to aim the free camera (`camera ex ey ez tx ty tz`).
- Models are small (a mine is ~10 units): at 720p frame them from ~60-100 units.

## Driving

- `./a2test drive` with shell variables or loops in one command was refused by the
  sandbox in this setup; put the sequence in a small script (`cmds.sh SESSION "scene
  cmd" "sleep 2" "shot NAME" ...`) and run it with `bash <script> <literal args>`.
- A scenario's `Setup:` script runs from the a2test checkout (the real worktree), not
  from `--install`: a scene file that exists only in your copy is not found. Use an
  existing scene (`specials`) and `Scene "spawn ..."` steps instead. `a2test run`
  takes a scenario by path.
- For a stock comparison, `--install` a reflink copy of the pristine tree, never the
  pristine tree itself: `--install` runs its `./install`, which builds into it
  (`render3d/build`, cargo's `target`). Copying `base/` (24 MB) took a second.

## GPU and resolution

- The bench's GPU is the user's: a GPU hang (2026-10-15 18:24) took every bench game and
  the user's compositor down. One game per agent at a time, stop sessions as soon as
  you have what you need, `--res 1280x720` unless judging fine detail.
- `session start` takes about 2-3 minutes (clone, `a2mod stock`, both installs, ~30 s
  to the first tick). Start it with the tool's background mode and wait for `stop it:`
  in its output; a `( ... &)` subshell got killed with the shell.

## Survey

- `./a2test drive --session ID log Render3D.log | grep 'ordnance:'`, or the file itself in the clone's prefix
  (`~/.cache/a2test/sessions/<ID>/prefix/drive_c/users/steamuser/AppData/Local/armada2-refit/logs/Render3D.log`) (and, after the
  session stops, `<results>/interactive/<res>/logs/Render3D.log`).
