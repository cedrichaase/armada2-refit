# render3d's hull module forced to stand down: the engine's ships, pixel for pixel as with hulls=0

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: tools/engine/bench-mismatch.sh render3d.hulls.* testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 15 min

**Pending phase 8e (E5): render3d is not on its profile yet, so `Mismatch=` names no
hook of it and this scenario fails until it is.** The case is written now so that E5
has its acceptance: with every `render3d.hulls.*` hook forced to fail on a stock
executable, the hull module stands down and leaves the ships to the engine, which is the
same state as `[Modules] hulls=0`, so the frame equals
`roundD-base-port-showcase-hulls0` at RMSE 0. The staging is `port-showcase`'s.

Run with `--install .`. When E5 has landed, also expect Render3D.log to name the hooks
and the test (`engine: render3d.hulls.<hook> stands down: ... test mismatch`) and the
Refit screen's Graphics tab to show the note on the hulls row.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "engine: TEST: A2EngineTest.ini forces a mismatch for render3d.hulls.*".
4. Expect "Render3D.log" contains "test mismatch (A2EngineTest.ini)".
5. Wait 5 seconds.
6. Take a screenshot called "showcase".
7. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-hulls0" within rmse 0.
8. Expect "Render3D.log" not contains "PANIC".
9. Expect no crash.
