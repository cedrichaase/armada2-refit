# Over the minimap the cursor says the move goes to warp

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh warp
Timeout: 6 min

`ui/HUD.md`, "The minimap". A right click on the minimap orders the selection there
with `GO_WARP`, which a ship that can warp carries out at warp. With ships selected
that can warp, the UI's cursor over the minimap's square is the game's warp cursor
(`c_warp`, the blue target); with only ships that cannot, the move cursor (`c_move`,
green). Run it on a checkout: `./a2test run ui-hud-minimap-warp --install . -j 1`.

The scene (`testbench/scene/scenes/warp.ini`) is three Federation destroyers, which
have warp. At 1280x720 the minimap's square spans about 24..224 x 500..700.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Scene "select ship1 ship2 ship3".
5. Wait 2 seconds.
6. Move the mouse to 120,600.
7. Wait 1 seconds.
8. Take a screenshot called "warp cursor".
9. Expect "UI.log" contains "cursor: Mission draws c_warp".
10. Move the mouse to 640,300.
11. Wait 1 seconds.
12. Quit the game.
13. Expect no crash.
