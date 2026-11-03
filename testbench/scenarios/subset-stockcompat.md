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
5. Expect "QOLRules.log" not contains "items are paid when queued".
6. Scene "select yard1".
7. Press q.
8. Press x 3 times.
9. Wait 2 seconds.
10. Expect "QOLRules.log" not contains "paid dil".
11. Expect the game is still running.
12. Quit the game.
13. Expect no crash.
