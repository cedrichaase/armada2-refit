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

**The assimilation beam needs the target's corbomite** (`tech tgt gcorbom`). It only
lives while the cube can take a special weapon from its target, one the target's team
has available and the cube does not carry yet; `a2_borg01`'s tech tree makes the
Sovereign's corbomite impossible, so without the `tech` step it ended as soon as it was
fired (bench, 2026-10-16) and `assimilation-beam-close` showed the holding beam. It
holds about 6 s, until the cube has taken the weapon (`testbench/scene/README.md`,
"Tech tree"). The thin cyan line that shows from the cube in such shots is the cube's
own Borg phaser (`bbphaso`) firing at the enemy Sovereign, drawn as stock draws it.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
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
19. Scene "tech tgt gcorbom".
20. Expect scene "special holder gTechAssimBeam tgt" answers "fired".
21. Wait 2 seconds.
22. Expect "Render3D.log" contains "ordnance: drawing special techassimbeam (Cone)".
23. Take a screenshot called "assimilation-beam-close".
24. Scene "camera 3600 200 3175 3000 0 3175".
25. Scene "kill holder".
26. Wait 5 seconds.
27. Scene "spawn tractor bassault 3000 0 3220 180 1".
28. Scene "engines tractor off".
29. Scene "energy tractor on".
30. Expect scene "special tractor gtracbm tgt" answers "fired".
31. Wait 2 seconds.
32. Expect "Render3D.log" contains "ordnance: drawing special tracbm (Cone)".
33. Take a screenshot called "tractor-beam".
34. Scene "orbit tgt 60 15 700".
35. Wait 1 seconds.
36. Take a screenshot called "tractor-beam-close".
37. Scene "camera 3600 200 3175 3000 0 3175".
38. Scene "kill tractor".
39. Wait 5 seconds.
40. Scene "spawn assim bcruise2 3000 0 3220 180 1".
41. Scene "engines assim off".
42. Scene "energy assim on".
43. Scene "attack assim tgt".
44. Wait 25 seconds.
45. Expect scene "special assim gborgbor tgt" answers "fired".
46. Wait 2 seconds.
47. Expect "Render3D.log" contains "ordnance: drawing special borgbor (Cone)".
48. Scene "orbit tgt 60 15 700".
49. Wait 1 seconds.
50. Take a screenshot called "auto-assimilator-close".
51. Wait 1 seconds.
52. Take a screenshot called "auto-assimilator-close-2".
53. Scene "camera 3600 200 3175 3000 0 3175".
54. Scene "kill assim".
55. Wait 5 seconds.
56. Scene "kill tgt".
57. Wait 4 seconds.
58. Scene "spawn psitgt fbattle 3000 0 3000 0 2".
59. Scene "immortal psitgt on".
60. Scene "weapons psitgt off".
61. Scene "engines psitgt off".
62. Scene "spawn psi 8472_battle 3000 0 3220 180 1".
63. Scene "engines psi off".
64. Scene "energy psi on".
65. Expect scene "special psi gpsionicInsanity psitgt" answers "fired".
66. Wait 6 seconds.
67. Take a screenshot called "psionic-insanity".
68. Wait 1 second.
69. Scene "kill psi".
70. Wait 5 seconds.
71. Scene "camera 1700 60 1250 1700 0 1500".
72. Scene "goto warper 9000 0 1500 warp".
73. Wait 1 second.
74. Take a screenshot called "warp-1".
75. Take a screenshot called "warp-2".
76. Take a screenshot called "warp-3".
77. Expect "Render3D.log" contains "ordnance: warp flash: fgalaxy.odf".
78. Expect the game is still running.
79. Quit the game.
80. Expect no crash.
