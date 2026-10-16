# Our own mouse cursor in a mission: the engine's shape, drawn by the UI (UI.asi, [UI] Cursor=1)

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: ui/bench-widgets.sh
Timeout: 6 min

`ui/README.md`, "The cursor". The scene (`testbench/scene/scenes/ui-cursor.ini`) is the
player's Galaxy class (`mine`) with an enemy Klingon battleship (`foe`) 300 units to its
right, HUD and cursor on, and `UI.asi` with `Cursor=1`: the UI draws the cursor from the
sprite the engine chose and hides the engine's. At 1280x720 the pair sit at about 640,360
(`mine`) and 750,360 (`foe`) with the camera on `mine`. `UI.log` says each shape the UI
draws (`cursor: Mission draws c_attack`), so the steps read the log; the screenshots
show it.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "UI.log" contains "sprites from cursor.spr".
4. Expect "Scene.log" contains "scene ready".
5. Move the mouse to 900,250.
6. Wait 1 second.
7. Expect "UI.log" contains "Mission draws standard_cursor".
8. Note "with the Galaxy selected, empty space gives the move cursor".
9. Scene "select mine".
10. Move the mouse to 920,260.
11. Wait 1 second.
12. Expect "UI.log" contains "Mission draws c_move".
13. Take a screenshot called "cursor-move".
14. Note "over the enemy it is the attack cursor".
15. Move the mouse to 750,360.
16. Wait 1 second.
17. Expect "UI.log" contains "Mission draws c_attack".
18. Take a screenshot called "cursor-attack".
19. Check that each screenshot shows one cursor at the pointer: the green move ring in "cursor-move", the red attack crosshair over the enemy ship in "cursor-attack", and not two overlapping cursors (the engine's own is hidden).
