# Passthrough nebula's reference: the engine's map puffs on the forward chain

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: display/bench-mode.sh forward render3d/bench-ini.sh Modules:world=0 Modules:particles=0 Modules:hulls=0 Modules:effects=0 Modules:ordnance=0 Modules:specials=0 Modules:sky=0 Modules:overlays=0 nebula-cerulean Seed=7 Step=16 Pause=200
Timeout: 15 min

**Reference recorder; phase 11 deletes it** (with forward mode). Its frames are frozen as the baseline `p10-fwd-display-passthrough-nebula` (phase 10 X4, recorded from run 20261103-124516) and nothing compares against a live run of it any more; to re-record, `./a2test baseline record p10-fwd-display-passthrough-nebula --from <this run's case> --force`.

Run with `--install .`. Every render3d module off, so the engine draws the cerulean
nebula's puffs itself (the user's report of 2026-10-30, "the nebulae move more than in
stock"), here through the real chain (d3d8to9, DXVK on the GPU). A frame at tick 200,
the same paused frame 2 s later, then frames at ticks 202 and 220: the puffs' motion
from tick to tick.

    ./a2test run display-passthrough-nebula-forward --install .
    ./a2test baseline record p10-fwd-display-passthrough-nebula --from <that run's case> --force

1. Launch the game.
2. Wait up to 600 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "module particles: off".
4. Wait 5 seconds.
5. Take a screenshot called "t200".
6. Wait 2 seconds.
7. Take a screenshot called "t200b".
8. Expect shots "t200" and "t200b" to match within 0.
9. Scene "pause-at 202; resume".
10. Wait up to 60 seconds for "Scene.log" to say "paused at tick 202".
11. Wait 3 seconds.
12. Take a screenshot called "t202".
13. Scene "pause-at 220; resume".
14. Wait up to 60 seconds for "Scene.log" to say "paused at tick 220".
15. Wait 3 seconds.
16. Take a screenshot called "t220".
17. Expect no crash.
