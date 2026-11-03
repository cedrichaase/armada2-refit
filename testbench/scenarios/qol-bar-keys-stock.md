# Without the Refit bar QOL's position keys stand down and the stock keys work

Resolutions: 16:9
Mod: refit
Layers: qol
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh stations
Timeout: 10 min

`qol/README.md`, "QOL-1". Run it on a checkout: `./a2test run qol-bar-keys-stock --install .
-j 1`. Only QOL.asi is installed (no UI.asi, so no refit bar): `UI_BarCell` does not exist, so
the position keys do nothing and the stock bar and its keys work as in the stock game.
`QOL.log` says `bar: no refit bar, position keys off` the first time the bar's keyboard
step runs (when something of the player's is selected). The scene is `qol-settings-live`'s
(`yard1..3`, `ship1`, `ship2`). In stock, B opens a producer's build menu.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "QOL.log" contains "--- QOL BarKeys=1".
4. Expect "Scene.log" contains "scene ready".
5. Scene "select yard1".
6. Wait 2 seconds.
7. Expect "QOL.log" contains "bar: no refit bar, position keys off".
8. Press q.
9. Wait 1 second.
10. Expect "QOL.log" not contain "bar: key".
11. Note "the stock bar's own key B opens the build menu".
12. Press b.
13. Wait 1 second.
14. Take a screenshot called "stock build menu".
15. Check that the game's own button bar is drawn and shows a menu of ship icons to build, with no yellow letters in the buttons' corners.
16. Expect the game is still running.
17. Expect no crash.
