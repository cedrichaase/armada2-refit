# Our own mouse cursor in a mission: the engine's shape, drawn by the UI (UI.asi, [UI] Cursor=1)

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: ui/bench-widgets.sh
Timeout: 7 min

`ui/README.md`, "The cursor". The scene (`testbench/scene/scenes/ui-cursor.ini`) is the
player's Galaxy class (`mine`) with an enemy Klingon battleship (`foe`) 300 units to its
right, HUD and cursor on, and `UI.asi` with `Cursor=1`: the UI draws the cursor from the
sprite the engine chose and hides the engine's. At 1280x720 the pair sit at about 640,360
(`mine`) and 750,360 (`foe`) with the camera on `mine`. `UI.log` says each shape the UI
draws (`cursor: Mission draws c_attack`), so the steps read the log; the screenshots
show it. Then Scene.asi's `cursor off`, which skips the engine's draw of its cursor:
the UI reads that from the engine's draw (`cursor: the engine's cursor is held back`)
and draws none either (`Mission draws none`); `cursor on` brings both back.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
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
20. Note "Scene.asi's cursor off: the engine would show no cursor, so the UI draws none".
21. Scene "cursor off".
22. Wait 1 second.
23. Expect "UI.log" contains "the engine's cursor is held back".
24. Expect "UI.log" contains "Mission draws none".
25. Take a screenshot called "cursor-off".
26. Scene "cursor on".
27. Wait 1 second.
28. Expect "UI.log" contains "the engine's cursor is shown".
29. Take a screenshot called "cursor-on-again".
30. Check that "cursor-off" shows no cursor at all over the enemy ship at 750,360, and "cursor-on-again" shows the red attack crosshair there again, one cursor only.
