# A planet under the fog of war keeps render3d's sphere and surface

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh fog-planet
Timeout: 6 min

`render3d/README.md`, "overlays" (ghosts) and "world" (planets). Run it on a checkout:
`./a2test run render3d-planet-ghost --install .`. The scene
(`testbench/scene/scenes/fog-planet.ini`) parks the player's scout beside a class M
planet with the map's fog on, then sends it away, so the planet is left as a
fog-of-war ghost (a spirit of `Ghost::ghostList`). Stock draws the ghost through the
engine's own planet path: its low-polygon sphere and stock texture, dimmed. render3d
draws it on its own sphere with the planet's surface, dimmed by `[World] PlanetGhost`.
Judge the shots: "planet-seen" is the planet lit; in "planet-ghost" the same planet is
under the fog, as round and with the same surface, only darker; never a faceted
outline or the stock texture.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module world: on".
5. Take a screenshot called "planet-seen".
6. Scene "goto scout 6500 0 6500".
7. Wait 30 seconds.
8. Expect "Render3D.log" contains "world: planets: a fog-of-war ghost".
9. Take a screenshot called "planet-ghost".
10. Expect the game is still running.
11. Quit the game.
12. Expect no crash.
