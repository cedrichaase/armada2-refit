# render3d draws construction: the station rising, the welding, the worker bees

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh construction
Timeout: 12 min

`render3d/README.md`, "effects" (construction, bees). Run it on a checkout:
`./a2test run render3d-construction --install . --res 1280x720`. A construction ship
is ordered to build a starbase (`build`, `../scene/README.md`); `Render3D.asi` draws the
station being built (`BuildingEffect`) through the hulls module, face by face as stock
adds them, with welding arcs and cooling seams along its new edges, and the worker
bees (`WorkerBee`) as lit models welding with arcs of light. The log says each part
was drawn by us; the shots must show it, in both views. The rig starts the station
within seconds of the order and `fbase` takes 100 s to build (12192 faces), so the
scene is paused part-built, 50 s in, to be looked at.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Scene "build rig fbase 2300 0 2000".
5. Wait 50 seconds.
6. Expect "Render3D.log" contains "effects: bees: fbee drawn".
7. Expect "Render3D.log" contains "effects: construction: fbase at".
8. Scene "pause; orbit 2300 0 2000 160 30 300".
9. Wait 3 seconds.
10. Take a screenshot called "building".
11. Scene "hud on".
12. Wait 3 seconds.
13. Take a screenshot called "views".
14. Check that the 3D view shows a partly built space station (panels and girders, some missing) with small glowing welding lights or orange lines along its edges, not black and with no garbled or stretched geometry.
15. Scene "resume".
16. Wait 60 seconds.
17. Take a screenshot called "later".
18. Expect the game is still running.
19. Expect "Render3D.log" not contains "PANIC".
20. Quit the game.
21. Expect no crash.
