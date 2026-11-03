# render3d: the specials module off at load, switched on live, draws a fresh effect

Resolutions: 1280x720
Mod: refit
Overlay: Render3D.Modules:specials=0
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh specials
Timeout: 10 min

`render3d/README.md`, "specials". The history-bearing half of the live parity of
`render3d-live-hulls` (phase 7, K7): with `specials=0` at load the engine draws the
special-object effects, so the module logs no layer; switched on live, an effect fired
after that is the module's, from its first frame on (its age counted from when it was
first seen, so nothing in flight is needed). The pixels of this are `render3d-specialfx`'s.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module specials: off".
5. Scene "camera 3150 90 3120 3000 0 3080".
6. Scene "spawn ovr bspecial 3000 0 3300 180 1".
7. Scene "engines ovr off".
8. Scene "energy ovr on".
9. Wait 2 seconds.
10. Expect scene "special ovr goverrid tgt" answers "fired".
11. Wait 4 seconds.
12. Expect "Render3D.log" not contains "first layer drawn".
13. Scene "kill ovr".
14. Wait 15 seconds.
15. Overlay --unset Render3D.Modules:specials
16. Wait up to 20 seconds for the logs the next steps expect.
17. Expect "Render3D.log" contains "settings: applied Modules.specials=1".
18. Scene "spawn ovr2 bspecial 3000 0 3300 180 1".
19. Scene "engines ovr2 off".
20. Scene "energy ovr2 on".
21. Wait 2 seconds.
22. Expect scene "special ovr2 goverrid tgt" answers "fired".
23. Wait 4 seconds.
24. Expect "Render3D.log" contains "first layer drawn: Override over fbattle".
25. Take a screenshot called "override-live".
26. Scene "weapons ovr2 off".
27. Overlay Render3D.Modules:specials=0
28. Wait up to 20 seconds for the logs the next steps expect.
29. Expect "Render3D.log" contains "settings: applied Modules.specials=0".
30. Wait 3 seconds.
31. Take a screenshot called "override-off".
32. Expect "Render3D.log" not contains "PANIC".
33. Expect no crash.
