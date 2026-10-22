# Two launches of one scene, paused at one tick, draw the same frame

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation
Timeout: 15 min

`testbench/README.md`, "A frame that repeats". Run it on a checkout:
`./a2test run bench-determinism --install .`. The `showcase` fleet action (five ships
firing at two Borg cubes in front of a planet) with the plugins' bench clock
(`Clock: bench`: render3d and the UI animate on the simulation's time), the engine's
random sequences seeded (`Seed=7`), every tick 16 ms long (`Step=16`) and the
simulation paused 200 ticks after the scene is built (`Pause=200`). The paused frame must stand still, and a second launch of the
same clone must draw the same picture, so a later port of the renderer can be checked
against it by a pixel diff.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Wait 5 seconds.
5. Take a screenshot called "first".
6. Wait 3 seconds.
7. Take a screenshot called "first again".
8. Expect shots "first" and "first again" to match within 0.
9. Stop the game.
10. Launch the game.
11. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
12. Wait 5 seconds.
13. Take a screenshot called "second".
14. Expect shots "first" and "second" to match within 0.5.
15. Expect no crash.
