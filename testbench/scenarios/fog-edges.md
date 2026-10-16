# Smooth fog and shroud edges drawn by Render3D.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh fog
Timeout: 6 min

`render3d/README.md`, "overlays" (fog). Run it on a checkout: `./a2test run fog-edges --install .`.
The scene (`testbench/scene/scenes/fog.ini`) is three of the player's ships with the
map's fog and shroud on, so each clears a circle of the shroud; the scout is then sent
off and leaves fog of war behind it. The log lines check that both layers were drawn by
the overlays module; the shots are for judging the look: the
edge between the flat grey shroud and open space, and between the darker fog and open
space, is a smooth curve with no steps a cell wide, and the map's own border, seen
from far off, stays straight and fully covered.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module overlays: on".
5. Expect "Render3D.log" contains "overlays: first shroud drawn".
6. Expect "Render3D.log" contains "overlays: first fog drawn".
7. Scene "goto scout 4200 0 3800".
8. Wait 30 seconds.
9. Take a screenshot called "fog-near".
10. Scene "orbit 2000 0 2000 200 60 7000".
11. Wait 3 seconds.
12. Take a screenshot called "fog-far".
13. Expect the game is still running.
14. Quit the game.
15. Expect no crash.
