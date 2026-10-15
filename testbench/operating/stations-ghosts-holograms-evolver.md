# Operating the game on the bench (hullsx: build ghost, height posts, holograms, Evolver)

Every command below was run from the real worktree (where `./a2test` lives), with
`--install <copy>` pointing at the agent's copy.

## Sessions

- A session with render3d and Scene.asi:
  `SCENE=stations ./a2test session start --res 16:9 --install <copy> --install <copy>/testbench/scene --args '-nointro a2_borg01'`
  The mission is up (HUD drawn, scene built) about 30-40 s after `session start` returns.
- **A stock reference in the same setup** (`--install` cannot go with `--mod stock`):
  start with `--no-launch`, set `Enable=0` under `[Render3D]` in the clone's
  `~/.cache/a2test/sessions/<ID>/game/Render3D.ini` (line 4), then
  `./a2test drive --session <ID> launch -nointro a2_borg01`. The engine then draws the
  whole 3D view as without the plugin (none of the replaced per-part plugins is installed any more,
  so this is the engine's own CPU path).
- `drive launch` refuses while the game runs; there is no restart command short of
  `drive quit` (an agent step) or a new session.

## The build ghost and its height post (stations scene)

The `stations` scene has no construction ship; spawn one and select it:

    ./a2test drive --session ID scene "spawn con fconst 1700 0 800"
    ./a2test drive --session ID scene "select con"

With `GridLayout.asi` the button bar is a 5x3 grid at the bottom left (1920x1080): the
first click on Q (362,885) opens the build menu, the second click on Q picks its first
station (the starbase). Then move the pointer into the view; the ghost follows the
pointer on the grid plane:

    ./a2test drive --session ID click 362 885
    ./a2test drive --session ID click 362 885
    ./a2test drive --session ID move 900 650
    ./a2test drive --session ID move 1550 780     # open space right of the yards: green footprint
    ./a2test drive --session ID scene "cursor off" # the cursor's icon covers the ghost otherwise

Over a station (e.g. `move 960 700`) the footprint is red (placement blocked).
A free camera keeps the placement going, and the ghost follows the pointer's ray
through the new camera (it moves when the camera moves):

    ./a2test drive --session ID scene "camera 2730 350 700 2730 0 1080"

At RTS zoom the ghost is ~100 px across under the pointer; crop and enlarge
(`magick shot.png -crop 360x300+1370+630 +repage -scale 300% crop.png`).

A right-click in the view (`drive click X Y --button 3`) cancels the placement.

## A hologram (Romulan holo-emitter)

    ./a2test drive --session ID scene "spawn rom rspecial 3000 0 3000 0 1"
    ./a2test drive --session ID scene "spawn foe kbattle 3300 0 3000 0 2"
    ./a2test drive --session ID scene "weapons foe off"
    ./a2test drive --session ID scene "special rom gholoemt foe"    # answers "slot 4, can fire, fired"
    ./a2test drive --session ID scene "special rom gholoemt foe"    # again: the first shot often does not take
    ./a2test drive --session ID scene "orbit foe 200 25 120"

The Klingon then shows as the Romulan's class (`rspecial`) for a while (the Klingon is
`TimedHoloEffect`'s target; the area cannon's wave is a purple ring). The holo-generator
(`ghologen`, on `fincursion`) is the other way round: the ship disguises itself as its
target's class.

## The Evolver (8472) evolving

The player is the Federation on `a2_borg01`, yet a spawned 8472 Evolver of team 1 can
evolve at once (no research needed):

    ./a2test drive --session ID scene "spawn evo 8472_passive 3400 0 2600 0 1"
    ./a2test drive --session ID scene "select evo"
    ./a2test drive --session ID scene "center evo"
    ./a2test drive --session ID click 242 590     # Q: its evolve menu (720p)
    ./a2test drive --session ID click 242 590     # Q again: the first entry, the Transmuter
    ./a2test drive --session ID scene "camera 3400 120 2400 3400 0 2600"

The cocoon (`8472_cocoon.sod`) appears at once, grows over ~5 s, bursts into debris,
and the Transmuter replaces the Evolver (a new object: an `orbit evo` camera then loses
its target and drifts). Shots once a second catch the whole thing. The cocoon is silvery
and solid in stock.

## Height posts

A ship above the grid's plane under the pointer gets a post from `mNormalModeRender`:
`scene "spawn high fdestroy 1700 300 1300 0 1"`, `scene "engines high off"`, then point
at it. A move order on the plane (`select`, right-click) gives a zero-length post at the
move target.

## Traps

- `drive key Escape` during placement opens the in-game OPTIONS menu (not "cancel the
  placement"); Scene.asi then stops reading commands ("Scene.cmd is not being read")
  until "Return to Game" (bottom left) is clicked.
- 720p (`--res 1280x720`) is the default now; the button grid's Q is at (242,590) there,
  and `move 1030 520` puts the ghost in open space right of the yards.
- Live tuning: append `[Hulls]` / `Reload=1` and the keys to the clone's
  `Render3D.ini`; the log says `hulls: [Hulls] read again` when a change is seen.

## Writing the scenario (render3d-hulls-extras)

- `Scene "spawn X ...; select X"` on one line selects nothing: the object is not there
  yet. Spawn, `Wait 1 second.`, then select.
- Clicks on the button bar 0.5 s apart (a scenario's default pace) miss the second
  menu; put `Wait 1 second.` after each click and move.
- Button-bar coordinates in hud space work at any 16:9 size: Q is `hud 403,983`; the
  ghost's open spot `hud 1717,867`.
- The holo-emitter: fire it three times 3 s apart and wait 6 s; one or two shots
  sometimes leave no hologram.
- `--no-claude` is fine: the scenario has no judged or agent steps.
- The match scenario (`render3d-match --res 1280x720 --no-claude`) passes and its log's
  `survey:` lines list what no module draws.

## The GPU is the user's

A GPU hang in one bench game (18:24, 2026-10-15) killed every bench game and the user's
desktop compositor. Run one game at a time, stop sessions as soon as you have the shots,
and keep every new shader pass bounded (fixed loop counts, capped draw counts).
