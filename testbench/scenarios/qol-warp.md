# A long move on the map goes to warp (QOL.asi, WarpDistance=)

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh warp
Timeout: 6 min

`qol/README.md`, QOL-8. Run it on a checkout: `./a2test run qol-warp --install .`.
The scene (`testbench/scene/scenes/warp.ini`) is three Federation destroyers at
2900..3100, 0, 3000, with the camera on the middle one. Zoomed all the way out at
1920x1080, a pixel is about 1.75 units, so the right edge of the screen is about
1600 units from the ships: past the default `WarpDistance=1100`. A point 190 px to the
right is about 330 units away, and stays an ordinary move. The two hover shots show
the cursor: stock's green move ring near, the game's blue warp ring far.

Measured by hand on the bench (2026-10-07): the ordinary move runs at about 240 units
a second, the far one peaks at about 700. The scenario does not assert a position:
warp spin-up makes the arrival time vary, and an early version that expected arrival
within 4 seconds failed with the warp cursor showing. Judge the hover shots.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "QOL.log" contains "goes to warp, patched".
4. Expect "Scene.log" contains "scene ready".
5. Press ctrl+Down 30 times.
6. Scene "select ship1 ship2 ship3".
7. Move the mouse to 1150,540.
8. Take a screenshot called "hover-near".
9. Move the mouse to 1850,540.
10. Take a screenshot called "hover-far".
11. Right-click at 1850,540.
12. Wait 8 seconds.
13. Take a screenshot called "after-move".
14. Expect the game is still running.
15. Quit the game.
16. Expect no crash.
