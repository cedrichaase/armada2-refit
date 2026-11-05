# A construction ship does not block its own blueprint (QOL.asi)

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh construction
Timeout: 10 min

`qol/README.md`, QOL-11. Run it on a checkout: `./a2test run qol-rig-ignores-self
--install . --res 1280x720`. The `construction` scene; each case spawns its own
construction ship far from the rest and centres the game's camera on it, so at 1280x720
and zoom 0.3 the ship is at 640,360 and the build menu's first button (Q) at 248,596. The
build menu is opened by two clicks on Q (the starbase), the ghost follows the pointer, and
a left click places it. `ConstructionRig::GetAction` is asked every frame; QOL.log says
`rig: the blueprint's spot is clear|blocked` (with `(the construction ship ignored)`
while `RigIgnoresSelf` is on) when the answer changes, and Render3D.log says
`effects: construction: fbase at X,Y,Z` once the rig builds.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "QOL.log" contains "a construction ship does not block its own blueprint, patched".
4. Expect "Scene.log" contains "scene ready".
5. Scene "hud on; grid on; camera rts"
6. Note "the rig over its own site: the ghost is clear, the order is sent, the rig builds"
7. Scene "spawn rig2 fconst 3200 0 3200 90"
8. Wait 2 seconds.
9. Scene "select rig2; center rig2; zoom 0.3"
10. Wait 3 seconds.
11. Move the mouse to 248,596
12. Wait 2 seconds.
13. Click at 248,596
14. Wait 1 second.
15. Click at 248,596
16. Wait 1 second.
17. Move the mouse to 640,360
18. Wait up to 20 seconds for "QOL.log" to say "the blueprint's spot is clear (the construction ship ignored)"
19. Take a screenshot called "ghost-over-rig"
20. Click at 640,360
21. Wait up to 60 seconds for "Render3D.log" to say "construction: fbase at 3200,"
22. Note "an own warship on the site: still blocked, and the click sends nothing"
23. Scene "spawn rig3 fconst 3200 0 4800 90; spawn war fgalaxy 3240 0 4800 90"
24. Wait 2 seconds.
25. Scene "select rig3; center rig3"
26. Wait 3 seconds.
27. Move the mouse to 248,596
28. Wait 2 seconds.
29. Click at 248,596
30. Wait 1 second.
31. Click at 248,596
32. Wait 1 second.
33. Move the mouse to 640,360
34. Wait up to 20 seconds for "QOL.log" to say "the blueprint's spot is blocked (the construction ship ignored)"
35. Take a screenshot called "ghost-over-warship"
36. Click at 640,360
37. Wait 10 seconds.
38. Expect "Render3D.log" not contains "construction: fbase at 3200,3,48"
39. Right-click at 640,360
40. Note "RigIgnoresSelf=0, live: the rig blocks its own ghost again, as in stock"
41. Overlay QOL.QOL:RigIgnoresSelf=0
42. Wait up to 20 seconds for "QOL.log" to say "applied QOL.RigIgnoresSelf=0"
43. Scene "spawn rig4 fconst 5200 0 3200 90"
44. Wait 2 seconds.
45. Scene "select rig4; center rig4"
46. Wait 3 seconds.
47. Move the mouse to 248,596
48. Wait 2 seconds.
49. Click at 248,596
50. Wait 1 second.
51. Click at 248,596
52. Wait 1 second.
53. Move the mouse to 640,360
54. Wait up to 20 seconds for "QOL.log" to say "the blueprint's spot is blocked"
55. Take a screenshot called "stock-ghost-over-rig"
56. Click at 640,360
57. Wait 10 seconds.
58. Expect "Render3D.log" not contains "construction: fbase at 5200,"
59. Expect the game is still running.
60. Quit the game.
61. Expect no crash.
