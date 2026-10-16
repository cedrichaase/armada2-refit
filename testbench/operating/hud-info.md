# The UI's info panel on the bench (UI.asi, [HUD] Info=1)

What was learned staging every face of the info panel (`ui/HUD.md`, "The info
panel"), 2026-10-16 at 1280x720.

- **The setup**: `ui/bench-hud-info.sh <state.json> [S.K=V ...]` puts `Scene.asi`
  with the `ui-hud-info` scene and `UI.asi` with `Info=1` and `Bar=1` into the clone
  (GridLayout.asi taken out). `session start --no-launch`, the script, then `drive
  launch -nointro a2_borg01`; wait for `scene ready` in `Scene.log`. To try a new
  build: `drive quit`, the script again, `drive launch ...`.
- **Selecting is easiest through the scene**: `scene "select mine"`, `scene "select d1
  d2 d3"` for a group. The panel follows within a frame; `UI.log` names each object
  once ("info: Single <name> | <class> | ...").
- **The HUD's race on `a2_borg01` changes from launch to launch** (Federation, Borg
  and Klingon came up on one afternoon), and with `Theme=auto` the look with it.
  `UI.log` says which ("race: the HUD's race is \"borg\""). For clicks at fixed places,
  fix the look with `UI.Theme=borg` (the panel's insides differ per form: LCARS
  frames take room at the top and left).
- **Damage**: the scene's Klingons (`foe`, `foe2`, engines on) went for the nearest
  destroyer, `d1`, rather than their `Attack=` target, on every launch; `d1` cannot die
  (the scene's `Immortal` is 1 by default) and after a minute shows red systems, hull
  around 10% and shields at 0%. Spawned attackers (`spawn k2 kbattle ... 2`, `attack
  k2 hurt`) also chose `d1`.
- **The queue**: select `base`, then `key q` opens its build list and each `key q`
  queues a construction ship; five presses about half a second apart give four in the
  queue. A construction ship takes about ten seconds, so hover and click a slot soon
  after (slot 3 of the Borg theme at 542,676).
- **Shift-click cannot be tested here**: a2input's `keydown shift` before a click
  does not reach the game, neither the UI (`GetKeyState`) nor the game's own Shift
  binding: a stock Shift-click on a ship in the 3D view does not add it to the
  selection either. `drive` has no modifier click; code that holds Shift
  through `bench.cli._current(sid).input('keydown shift')` showed the same.
- **Esc in a mission opens OPTIONS** and the scene stops reading commands until
  "Return to Game"; don't press it to "clear" anything.
- **A relaunch right after `drive quit` once came up black and died while loading**
  (a2test also reported the game playing to a real audio output); a plain `drive
  launch` again worked.
