# A release with every layer: a mission, the HUD and bar, the logs of every plugin

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 12 min

Phase 9, W-PKG (`publish/README.md`, "Per-layer selection"). The default install: every layer
the release offers, so no `Layers:` header. A mission with the `stations` scene, the UI's HUD
and button bar, and the log of every plugin that is in. Run it with
`./a2test run subset-full --install .`; the five other `subset-*` scenarios leave layers out
with `Layers:`.

1. Launch the game.
2. Wait up to 90 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "clock".
5. Expect "UI.log" contains "mission: frames start".
6. Expect "UI.log" contains "bar: ours".
7. Expect "QOLRules.log" contains "items are paid when queued".
8. Take a screenshot called "full".
9. Expect the game is still running.
10. Quit the game.
11. Expect no crash.
