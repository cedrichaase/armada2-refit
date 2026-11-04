# No UI layer: the engine's front-end dialogs and HUD in front of render3d's 3D view

Resolutions: 1280x720
Mod: refit
Layers: display render3d
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 12 min

Without `ui` the front-end dialogs and the HUD are the engine's own, drawn through Display8's
passthrough, and render3d draws the 3D view. Nothing of the UI runs: no UI.log lines of a
hosted menu or a mission frame.

1. Launch the game.
2. Wait up to 90 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module hulls: loaded".
5. Take a screenshot called "stock-hud".
6. Expect the game is still running.
7. Quit the game.
8. Expect no crash.
