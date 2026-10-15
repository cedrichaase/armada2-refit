# The warp flash and the special weapons, drawn by Render3D.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh specials
Timeout: 8 min

`render3d/README.md`, "ordnance" (recipes, the warp flash). Run it on a checkout: `./a2test run specials --install .`. The
scene (`testbench/scene/scenes/specials.ini`) is an enemy Sovereign (`tgt`) that cannot
die and does not fire, and a Galaxy (`warper`) off to one side. A Borg cube fires its
holding beam at the Sovereign, an Assault ship its tractor beam, a Species 8472
battleship psionic insanity at a second Sovereign in the first's place, and the Galaxy goes to warp. The log lines check that the module is on
and that the beams and the flash were drawn by it (psionic insanity's wave is
not asserted: fired from a scenario it often sets off none; see `render3d/README.md`); the shots are for judging the look: a glowing
green beam with pulses where stock has a cone of thin green rays, a soft blue beam with
glows at both ends where stock has a thin double line, a pink band of light with rays
running out where stock has a textured disc, and a white flash with a blue lens streak
and a ring where stock has a soft white star.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module ordnance: on".
5. Scene "spawn holder bbattle1 3000 0 3220 180 1".
6. Scene "engines holder off".
7. Scene "energy holder on".
8. Expect scene "special holder gholdbm tgt" answers "fired".
9. Wait 2 seconds.
10. Expect "Render3D.log" contains "ordnance: drawing special holdbm (Beam)".
11. Take a screenshot called "holding-beam".
12. Scene "kill holder".
13. Wait 5 seconds.
14. Scene "spawn tractor bassault 3000 0 3220 180 1".
15. Scene "engines tractor off".
16. Scene "energy tractor on".
17. Expect scene "special tractor gtracbm tgt" answers "fired".
18. Wait 2 seconds.
19. Expect "Render3D.log" contains "ordnance: drawing special tracbm (Beam)".
20. Take a screenshot called "tractor-beam".
21. Scene "kill tractor".
22. Wait 5 seconds.
23. Scene "kill tgt".
24. Wait 4 seconds.
25. Scene "spawn psitgt fbattle 3000 0 3000 0 2".
26. Scene "immortal psitgt on".
27. Scene "weapons psitgt off".
28. Scene "engines psitgt off".
29. Scene "spawn psi 8472_battle 3000 0 3220 180 1".
30. Scene "engines psi off".
31. Scene "energy psi on".
32. Expect scene "special psi gpsionicInsanity psitgt" answers "fired".
33. Wait 6 seconds.
34. Take a screenshot called "psionic-insanity".
35. Wait 1 second.
36. Scene "kill psi".
37. Wait 5 seconds.
38. Scene "camera 1700 60 1250 1700 0 1500".
39. Scene "goto warper 9000 0 1500 warp".
40. Wait 1 second.
41. Take a screenshot called "warp-1".
42. Take a screenshot called "warp-2".
43. Take a screenshot called "warp-3".
44. Expect "Render3D.log" contains "ordnance: warp flash: fgalaxy.odf".
45. Expect the game is still running.
46. Quit the game.
47. Expect no crash.
