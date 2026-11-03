# The C plugins' hooks forced to stand down: QOL, rules and Online run stock, the frame does not change

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: tools/engine/bench-mismatch.sh qol.*,qolrules.*,online.* testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 15 min

With every hook of QOL.asi, the rules plugin and Online.asi forced to fail on a stock
executable, each logs the hook standing down with the test reason and runs as stock;
none of them draws, so the frame equals `roundD-base-port-showcase` except the two
yellow view-frustum lines on the minimap: QOL's `view_distance` is not applied, so the
camera frustum the minimap draws is the stock one (149 px, rmse 2.04, the same in every
run). The scene and the rest of the HUD are identical, so the step allows rmse 2.5. The
Refit screen shows their notes on the Gameplay and Multiplayer rows (a judged step, run
once by hand; see `ui-menus-refit`).

Run with `--install .`.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Wait 5 seconds.
4. Take a screenshot called "showcase".
5. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 2.5.
6. Expect "QOL.log" contains "engine: TEST: A2EngineTest.ini forces a mismatch for qol.*, qolrules.*, online.*".
7. Expect "QOL.log" contains "stands down: ".
8. Expect "QOL.log" contains "test mismatch (A2EngineTest.ini)".
9. Expect "QOL.log" not contains "PANIC".
10. Expect no crash.
