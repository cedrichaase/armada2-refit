# Display8, render3d, the UI and the models: no QOL, stock pan

Resolutions: 1280x720
Mod: refit
Layers: display render3d ui models
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 12 min

No `qol`, `rules` or `online`: no QOL.asi, so the right-drag pan speed, the control groups
and the selection are stock. The picture is the full install's.

1. Launch the game.
2. Wait up to 90 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module hulls: loaded".
5. Expect "UI.log" contains "mission: frames start".
6. Take a screenshot called "visuals".
7. Expect the game is still running.
8. Quit the game.
9. Expect no crash.
