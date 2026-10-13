# Cloaked ships drawn as glass by Lighting.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh cloak
Timeout: 6 min

`lighting/README.md`, "Cloaked ships". Run it on a checkout: `./a2test run cloak --install .`.
The scene (`testbench/scene/scenes/cloak.ini`) is the player's own Romulan warbird and
Klingon Bird of Prey in front of a class M planet and a Galaxy class. The warbird
cloaks, holds, and decloaks; the Bird of Prey cloaks at once. The log line checks that
a cloaked hull went through the glass path; the shots are for judging the look: halfway,
the hull going patch by patch behind a green front; cloaked, clear glass with a thin
bright edge, the planet and the Galaxy behind it a little bent and fringed in colour,
no opaque or flat-tinted hull; decloaked, the warbird as before.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Take a screenshot called "cloak-before".
5. Scene "cloak warbird on".
6. Wait 1 seconds.
7. Take a screenshot called "cloak-halfway".
8. Wait 8 seconds.
9. Expect scene "query" answers "warbird handle".
10. Expect scene "query" answers "cloak cloaked".
11. Expect "Lighting.log" contains "cloak: a cloaked hull drawn as glass at Flush".
12. Take a screenshot called "cloak-glass".
13. Scene "cloak bop now".
14. Scene "orbit warbird 300 10 230".
15. Wait 3 seconds.
16. Take a screenshot called "cloak-nebula".
17. Scene "orbit warbird 230 45 900".
18. Wait 3 seconds.
19. Take a screenshot called "cloak-rts".
20. Scene "orbit warbird 230 12 260".
21. Scene "cloak warbird off".
22. Wait 5 seconds.
23. Expect scene "query" answers "cloak visible".
24. Take a screenshot called "cloak-after".
25. Expect the game is still running.
26. Quit the game.
27. Expect no crash.
