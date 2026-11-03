# No render3d layer: the engine's 3D view through the passthrough, the UI's HUD over it

Resolutions: 1280x720
Mod: refit
Layers: display ui
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 12 min

Without `render3d` the 3D view is the engine's, drawn through Display8's passthrough; the
UI's HUD and button bar are drawn over it.

1. Launch the game.
2. Wait up to 90 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" not contains "clock".
5. Expect "UI.log" contains "mission: frames start".
6. Expect "UI.log" contains "bar: ours".
7. Take a screenshot called "stock-3d".
8. Expect the game is still running.
9. Quit the game.
10. Expect no crash.
