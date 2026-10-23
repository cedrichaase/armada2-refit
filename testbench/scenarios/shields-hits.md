# Shield hits drawn by Render3D.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh shields
Timeout: 6 min

`render3d/README.md`, "effects" (shields). Run it on a checkout: `./a2test run shields-hits --install .`.
The scene (`testbench/scene/scenes/shields.ini`) is a Galaxy class (phasers) and a
Klingon battleship (torpedoes) firing at a second Galaxy, which is healed every tick so
its shields hold. The log lines check that the hits were noted at the weapons' call sites and that hits of both
kinds (a beam and a shot) were picked up and drawn; the shots are for judging the look:
a pale blue mist about each hit, uneven like lit vapour, on the target's shield
ellipsoid where the weapons land and ending on a soft curve at its silhouette, a
near-white point where the shot lands, the hull under it lit, no hex pattern and no
flat shell; then a Klingon cruiser taking the same fire, its hits in strong green with
rays fanning out from each (`ART-DIRECTION.md`, "Shield hits").

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "effects: shields: the weapons' hits noted at: beam shot mine missile".
5. Scene "orbit target 160 20 170".
6. Wait up to 6 seconds for the logs the next steps expect.
7. Expect "Render3D.log" contains "effects: first shield hits drawn".
8. Expect "Render3D.log" contains "by a beam".
9. Expect "Render3D.log" contains "by a missile".
10. Take a screenshot called "shields-close".
11. Wait 2 seconds.
12. Take a screenshot called "shields-close-2".
13. Scene "orbit target 200 25 260".
14. Wait 2 seconds.
15. Take a screenshot called "shields-wide".
16. Scene "spawn ktgt kcruise1 2600 0 2350 180 2".
17. Scene "immortal ktgt on".
18. Scene "heal ktgt on".
19. Scene "weapons ktgt off".
20. Scene "engines ktgt off".
21. Scene "attack shooter ktgt".
22. Scene "attack klingon ktgt".
23. Scene "orbit ktgt 160 20 140".
24. Wait 6 seconds.
25. Take a screenshot called "shields-klingon".
26. Wait 2 seconds.
27. Take a screenshot called "shields-klingon-2".
28. Expect the game is still running.
29. Quit the game.
30. Expect no crash.
