# Passthrough's reference with every render3d module off, on the forward chain

Resolutions: 1280x720
Mod: refit
Clock: bench
Setup: display/bench-mode.sh forward render3d/bench-ini.sh Modules:effects=0 Modules:ordnance=0 Modules:specials=0 Modules:sky=0 Modules:overlays=0 Modules:particles=0 Modules:hulls=0 Modules:world=0
Timeout: 15 min

**Reference recorder; phase 11 deletes it** (with forward mode). Its frames are frozen as the baseline `p10-fwd-display-passthrough-all` (phase 10 X4, recorded from run 20261103-124333) and nothing compares against a live run of it any more; to re-record, `./a2test baseline record p10-fwd-display-passthrough-all --from <this run's case> --force`.

Run with `--install .`. `display-passthrough-all`'s mission with Display8 in forward
mode: every one of render3d's eight modules off, so the engine draws the whole scene
through the real chain. What `display-passthrough-all` is compared against (its text
says how).

1. Launch the game.
2. Wait for the main menu.
3. Click the single player emblem.
4. Wait 5 seconds.
5. Click "Borg Campaign".
6. Wait for "Strange Bedfellows".
7. Click "Strange Bedfellows".
8. Wait 1 second.
9. Click "O.K.".
10. Wait up to 300 seconds for the briefing.
11. Wait 3 seconds.
12. Click "OK".
13. Wait 2 seconds.
14. Click at 55,555.
15. Wait 5 seconds.
16. Take a screenshot called "borg07".
17. Expect "Render3D.log" contains "module hulls: off for missions".
18. Expect no crash.
