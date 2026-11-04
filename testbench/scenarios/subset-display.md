# Display8 alone: the front end, a mission, the cursor visible

Resolutions: 1280x720
Mod: refit
Layers: display
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 12 min

Only the core layers: the engine draws everything through Display8's passthrough. The
screenshot shows the stock mission with the engine's own cursor; a missing cursor would be
the stub drawing nothing for it.

1. Launch the game.
2. Wait up to 90 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Take a screenshot called "display-only".
5. Expect the game is still running.
6. Quit the game.
7. Expect no crash.
