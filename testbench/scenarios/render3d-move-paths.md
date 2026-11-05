# Move paths: a queue drawn for the selected ships, gone with the selection and the queue

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh warp
Timeout: 25 min

Issue #17. `render3d/README.md`, "Move paths", `qol/README.md`, QOL-10, and
`ui/HUD.md` (the minimap's point). Run it on a checkout: `./a2test run
render3d-move-paths --install .`. `qol-move-queue.md` checks the queues themselves (what
QOL sends and when); this case checks what render3d draws of them, and the few queue
facts that go with a shot. The bench cannot hold a key, so Tab is held with `Scene
"waypoint-key on"` and let go with `off`. On the `warp` scene, the middle destroyer
(`ship2`, at the middle of the screen) is selected first.

render3d draws a path only for the **selected** ships, from QOL's queue (export
`QOL_MoveQueue`; its log line ends in `(qol)`), as a very light grey, fine, dotted line
from the ship through the points ahead, with a bigger green dot at each point. What is
flown is gone from it. `N selected ship(s) on a path` is the log's count, so it goes to 0
when the ship is deselected while its queue still stands in QOL, and again when the queue
ends (Stop, a plain move).

The case: five points are queued with Tab, the count of points ahead following each
("held"; "released"). `ship1` is selected: the line says 0 selected ships on a path, QOL
has cleared nothing, "deselected" shows no path; `ship2` again draws it ("mid-flight"). A
plain right-click clears the queue and render3d with it ("cleared"). Then a go leg and,
from a Tab click on the minimap, a warp leg: "mixed" shows both, "go-leg" the ship on the
first leg (at impulse: QOL has not sent the warp run yet), "warp-leg" the ship on the
second after the warp run went out. Last, the bar's Stop clears a queue and the drawing
("stopped").

Judge the shots: the line is very light grey and fine, dotted, not stock's one-pixel green
line with boxes; the point dots are green and bigger than the line's dots; nothing is
drawn for a ship that is not selected; the part flown is not drawn.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "overlays: move paths 1".
5. Expect "Render3D.log" contains "overlays: move paths: QOL's queue found (version 1)".
6. Expect "QOL.log" contains "move queues: Tab points (and minimap points) queued per ship, flown at once, patched".
7. Scene "select ship2".
8. Expect scene "selection" answers "ship2".
9. Move the mouse to 840,360.
10. Scene "waypoint-key on".
11. Right-click at 840,360.
12. Wait up to 10 seconds for "QOL.log" to say "move queue: run sent: go, 1 point(s) from point 1".
13. Wait up to 10 seconds for "Render3D.log" to say "1 selected ship(s) on a path".
14. Right-click at 840,200.
15. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 2 (go)".
16. Wait up to 10 seconds for "Render3D.log" to say "2 point(s) ahead (qol)".
17. Right-click at 520,200.
18. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 3 (go)".
19. Wait up to 10 seconds for "Render3D.log" to say "3 point(s) ahead (qol)".
20. Right-click at 400,420.
21. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 4 (go)".
22. Right-click at 520,300.
23. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 5 (go)".
24. Expect "Render3D.log" contains "4 point(s) ahead (qol)".
25. Take a screenshot called "held".
26. Scene "waypoint-key off".
27. Wait up to 10 seconds for "QOL.log" to say "move queue: key let go, the points are queued already".
28. Wait 1 seconds.
29. Take a screenshot called "released".
30. Scene "select ship1".
31. Wait up to 10 seconds for "Render3D.log" to say "0 selected ship(s) on a path".
32. Expect "QOL.log" not contain "move queue: cleared".
33. Take a screenshot called "deselected".
34. Scene "select ship2".
35. Wait up to 10 seconds for "Render3D.log" to say "1 selected ship(s) on a path".
36. Wait 1 seconds.
37. Take a screenshot called "mid-flight".
38. Right-click at 640,300.
39. Wait up to 10 seconds for "QOL.log" to say "move queue: cleared for 1 ship(s): move".
40. Wait up to 10 seconds for "Render3D.log" to say "0 selected ship(s) on a path, 0 point(s) ahead".
41. Wait 2 seconds.
42. Take a screenshot called "cleared".
43. Scene "waypoint-key on".
44. Right-click at 840,360.
45. Wait up to 10 seconds for "QOL.log" to say "move queue: run sent: go, 1 point(s) from point 1".
46. Right-click at 840,200.
47. Right-click at 520,200.
48. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 3 (go)".
49. Right-click at 180,530.
50. Wait up to 10 seconds for "UI.log" to say "queued (QOL)".
51. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 4 (warp)".
52. Wait up to 10 seconds for "Render3D.log" to say "4 point(s) ahead (qol)".
53. Take a screenshot called "mixed".
54. Scene "waypoint-key off".
55. Wait 1 seconds.
56. Expect "QOL.log" not contain "next run sent: warp".
57. Take a screenshot called "go-leg".
58. Wait up to 60 seconds for "QOL.log" to say "move queue: next run sent: warp, 1 point(s) from point 4, 1 ship(s)".
59. Wait 2 seconds.
60. Take a screenshot called "warp-leg".
61. Wait up to 90 seconds for "QOL.log" to say "done, 1 ship(s)".
62. Wait up to 10 seconds for "Render3D.log" to say "0 selected ship(s) on a path".
63. Press Home.
64. Scene "select ship1".
65. Scene "waypoint-key on".
66. Right-click at 900,200.
67. Right-click at 900,450.
68. Wait up to 10 seconds for "QOL.log" to say "move queue: run sent: go, 2 point(s) from point 1, 1 ship(s)".
69. Wait up to 10 seconds for "Render3D.log" to say "1 selected ship(s) on a path".
70. Scene "waypoint-key off".
71. Wait 2 seconds.
72. Click at 249,640.
73. Wait up to 10 seconds for "QOL.log" to say "move queue: cleared for 1 ship(s): stop".
74. Wait up to 10 seconds for "Render3D.log" to say "0 selected ship(s) on a path".
75. Wait 2 seconds.
76. Take a screenshot called "stopped".
77. Expect the game is still running.
78. Quit the game.
79. Expect no crash.
