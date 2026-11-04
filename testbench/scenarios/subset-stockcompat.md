# Everything but the rules layer: a build is charged at start, as in stock

Resolutions: 1280x720
Mod: refit
Layers: display render3d ui qol online cutscenes models
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 12 min

Without `rules` there is no QOLRules.asi, so no rules log and nothing is paid at the queue:
the first item is charged when it starts, as in stock (`rules/README.md`, QOL-7). The other
layers are as installed. A player in this state plays a network game as a stock player
does (the host plays stock rules for everyone).

1. Launch the game.
2. Wait up to 90 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "UI.log" contains "mission: frames start".
5. Scene "select yard1".
6. Press q.
7. Press x 3 times.
8. Wait 2 seconds.
9. Expect the game is still running.
10. Quit the game.
11. Expect no crash.
