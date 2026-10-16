# The selection overlay over a damaged group, and the minimap's fog, at 1080 lines

Resolutions: 1920x1080
Mod: refit
Launch: -nointro a2_borg01
Setup: ui/bench-asi.sh --no-grid --scene fog HUD.Minimap=1 HUD.Selection=1 HUD.UnitView=1
Timeout: 10 min

`ui/HUD.md`, "The selection overlay" and "The minimap". The scene is `fog`
(`testbench/scene/scenes/fog.ini`): three of the player's Federation ships (`ship1` a
destroyer, `ship2` a Galaxy, `scout`) with the map's fog and shroud on. A Klingon
battleship is spawned for the enemy and attacks `ship1`, which is not healed, so its
hull gauge runs down. The three are selected and made group 1 (Ctrl+1).

The scene's teams have no map colour (no game setup), so the minimap colours their
blips by relation: own green, enemy red.

At 1920x1080 the minimap's square is about 27..327 x 753..1053.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "UI.log" contains "minimap: ours".
5. Expect "UI.log" contains "selection: ours".
6. Scene "spawn bad kbattle 2600 0 2600 0 2".
7. Wait 2 seconds.
8. Scene "attack bad ship1".
9. Scene "select ship1 ship2 scout".
10. Press ctrl+1.
11. Move the mouse to 1100,150.
12. Wait 15 seconds.
13. Take a screenshot called "group".
14. Check that three ships near the middle of the screen each have four small corner brackets around them, a small box with the number 1 at the left of the brackets, and thin bars above them, and that the bars of one of the ships (the one being fired at) are much shorter or red.
15. Scene "goto scout 4200 0 3000".
16. Wait 20 seconds.
17. Take a screenshot called "fog".
18. Check that the minimap at the bottom left is mostly a flat grey-blue with a black cleared area around a few green dots, a trail of darker area where the scout passed, and a white outline of the camera's view.
19. Right-click at 200,880.
20. Wait 2 seconds.
21. Expect "UI.log" contains "minimap: order".
22. Expect the game is still running.
