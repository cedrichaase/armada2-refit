# The 3D view through render3d

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh showcase
Timeout: 6 min

`render3d/README.md`. Run it on a checkout: `./a2test run render3d --install .`. The
`showcase` scene, a fleet firing at two Borg cubes in front of a planet, drawn through
`Render3D.asi`: both calls of `Armada_RenderAllOurStuff` go through it, the main view
and, once the HUD is on, the action camera's inset. The log lines check that the frame
made its targets and that each view was drawn; the shots are for judging the look
against stock: the same picture, with light past white blooming (weapon impacts, the
warp core, the hits on the shields), and edges smoothed by the scene target's MSAA.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "frame: both views come through render3d".
5. Expect "Render3D.log" contains "frame: targets".
6. Expect "Render3D.log" contains "main view".
7. Take a screenshot called "showcase".
8. Scene "orbit cube1 160 10 380".
9. Wait 3 seconds.
10. Take a screenshot called "close".
11. Scene "hud on".
12. Scene "select ente".
13. Wait up to 5 seconds for the logs the next steps expect.
14. Expect "Render3D.log" contains "inset view".
15. Take a screenshot called "hud-and-inset".
16. Wait 20 seconds.
17. Expect the game is still running.
18. Quit the game.
19. Expect no crash.
