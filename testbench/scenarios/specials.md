# The warp flash and the special weapons, drawn by Render3D.asi

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh specials
Timeout: 8 min

`render3d/README.md`, "ordnance" (recipes, the warp flash). Run it on a checkout: `./a2test run specials --install .`. The
scene (`testbench/scene/scenes/specials.ini`) is an enemy Sovereign (`tgt`) that cannot
die and does not fire, and a Galaxy (`warper`) off to one side. A Borg cube fires its
holding beam at the Sovereign, then its assimilation beam, an Assault ship its tractor beam, an assimilator its auto-assimilator, a Species 8472
battleship psionic insanity at a second Sovereign in the first's place, and the Galaxy goes to warp. The log lines check that the module is on
and that the beams and the flash were drawn by it (psionic insanity's wave is
not asserted: fired from a scenario it often sets off none; see `render3d/README.md`); the shots are for judging the look: a glowing
green cone of light in vapour, widening from a bright point on the cube to take in
most of the Sovereign, with lengthwise streaks, no core, and a faint green wash on the
Sovereign's hull (the holding and assimilation beams); the same in blue for the tractor, and in blue with a fan of green now and then for the
auto-assimilator (`ART-DIRECTION.md`, "Tractor and assimilation beams"), a pink band of light with rays
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
10. Expect "Render3D.log" contains "ordnance: drawing special holdbm (Cone)".
11. Take a screenshot called "holding-beam".
12. Scene "orbit tgt 60 15 700".
13. Wait 1 seconds.
14. Take a screenshot called "holding-beam-close".
15. Scene "orbit tgt 60 65 900".
16. Wait 1 seconds.
17. Take a screenshot called "holding-beam-above".
18. Wait 4 seconds.
19. Expect scene "special holder gTechAssimBeam tgt" answers "fired".
20. Wait 2 seconds.
21. Expect "Render3D.log" contains "ordnance: drawing special techassimbeam (Cone)".
22. Take a screenshot called "assimilation-beam-close".
23. Scene "camera 3600 200 3175 3000 0 3175".
24. Scene "kill holder".
25. Wait 5 seconds.
26. Scene "spawn tractor bassault 3000 0 3220 180 1".
27. Scene "engines tractor off".
28. Scene "energy tractor on".
29. Expect scene "special tractor gtracbm tgt" answers "fired".
30. Wait 2 seconds.
31. Expect "Render3D.log" contains "ordnance: drawing special tracbm (Cone)".
32. Take a screenshot called "tractor-beam".
33. Scene "orbit tgt 60 15 700".
34. Wait 1 seconds.
35. Take a screenshot called "tractor-beam-close".
36. Scene "camera 3600 200 3175 3000 0 3175".
37. Scene "kill tractor".
38. Wait 5 seconds.
39. Scene "spawn assim bcruise2 3000 0 3220 180 1".
40. Scene "engines assim off".
41. Scene "energy assim on".
42. Scene "attack assim tgt".
43. Wait 25 seconds.
44. Expect scene "special assim gborgbor tgt" answers "fired".
45. Wait 2 seconds.
46. Expect "Render3D.log" contains "ordnance: drawing special borgbor (Cone)".
47. Scene "orbit tgt 60 15 700".
48. Wait 1 seconds.
49. Take a screenshot called "auto-assimilator-close".
50. Wait 1 seconds.
51. Take a screenshot called "auto-assimilator-close-2".
52. Scene "camera 3600 200 3175 3000 0 3175".
53. Scene "kill assim".
54. Wait 5 seconds.
55. Scene "kill tgt".
56. Wait 4 seconds.
57. Scene "spawn psitgt fbattle 3000 0 3000 0 2".
58. Scene "immortal psitgt on".
59. Scene "weapons psitgt off".
60. Scene "engines psitgt off".
61. Scene "spawn psi 8472_battle 3000 0 3220 180 1".
62. Scene "engines psi off".
63. Scene "energy psi on".
64. Expect scene "special psi gpsionicInsanity psitgt" answers "fired".
65. Wait 6 seconds.
66. Take a screenshot called "psionic-insanity".
67. Wait 1 second.
68. Scene "kill psi".
69. Wait 5 seconds.
70. Scene "camera 1700 60 1250 1700 0 1500".
71. Scene "goto warper 9000 0 1500 warp".
72. Wait 1 second.
73. Take a screenshot called "warp-1".
74. Take a screenshot called "warp-2".
75. Take a screenshot called "warp-3".
76. Expect "Render3D.log" contains "ordnance: warp flash: fgalaxy.odf".
77. Expect the game is still running.
78. Quit the game.
79. Expect no crash.
