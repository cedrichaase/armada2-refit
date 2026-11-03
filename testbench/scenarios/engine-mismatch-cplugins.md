# The C plugins' hooks forced to stand down: QOL, rules and Online run stock, the frame does not change

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: tools/engine/bench-mismatch.sh qol.*,qolrules.*,online.* testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 15 min

**Pending phase 8d (E4): the C plugins are not on their profile yet, so `Mismatch=`
names no hook of theirs and the log steps fail until they are.** Written now so that E4
has its acceptance. With every hook of QOL.asi, the rules plugin and Online.asi forced
to fail on a stock executable, each logs the hook standing down with the test reason
and runs as stock; none of them draws, so the frame equals `roundD-base-port-showcase`
at RMSE 0. The Refit screen shows their notes on the Gameplay and Multiplayer rows (a
judged step, run once by hand after E4; see `ui-menus-refit`).

Run with `--install .`.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Wait 5 seconds.
4. Take a screenshot called "showcase".
5. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
6. Expect "QOL.log" contains "engine: TEST: A2EngineTest.ini forces a mismatch for qol.*,qolrules.*,online.*".
7. Expect "QOL.log" contains "stands down: ".
8. Expect "QOL.log" contains "test mismatch (A2EngineTest.ini)".
9. Expect "QOL.log" not contains "PANIC".
10. Expect no crash.
