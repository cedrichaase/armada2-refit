# Shield hits drawn by Shields.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh shields
Timeout: 6 min

`shields/README.md`. Run it on a checkout: `./a2test run shields-hits --install .`.
The scene (`testbench/scene/scenes/shields.ini`) is a Galaxy class (phasers) and a
Klingon battleship (torpedoes) firing at a second Galaxy, which is healed every tick so
its shields hold. The log lines check that every hook was patched and that hits of both
kinds (a beam and a shot) were picked up and drawn; the shots are for judging the look:
smooth blue glows and rings, no hex pattern, on the target's shield ellipsoid where the weapons land,
no flat green shell.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Shields.log" contains "DrawShield and RenderChunks patched; the weapons' hits: beam shot mine missile".
5. Scene "orbit target 160 20 170".
6. Wait 6 seconds.
7. Expect "Shields.log" contains "first shield hits drawn".
8. Expect "Shields.log" contains "by a beam".
9. Expect "Shields.log" contains "by a missile".
10. Take a screenshot called "shields-close".
11. Wait 2 seconds.
12. Take a screenshot called "shields-close-2".
13. Scene "orbit target 200 25 260".
14. Wait 2 seconds.
15. Take a screenshot called "shields-wide".
16. Expect the game is still running.
17. Quit the game.
18. Expect no crash.
