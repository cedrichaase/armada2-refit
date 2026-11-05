# Move queues: Tab points appended, flown a run at a time, ended by the player's orders

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh warp
Layers: qol ui
Timeout: 20 min

Issue #17, `qol/README.md`, QOL-10. Run it on a checkout: `./a2test run qol-move-queue
--install .`. The bench cannot hold a key, so Tab is held with `Scene "waypoint-key
on"` and let go with `off` (`render3d-move-paths.md`). On the `warp` scene, with the
middle destroyer (`ship2`) selected at first.

First three points are queued with Tab, the second while the ship flies to the first
(it is appended, and the order is sent again from where the ship is) and the third after
it (sent from point 2), and the queue is flown to its end. Then a Tab point in the 3D
view and a Tab click on the minimap: a warp leg, a second run that waits, goes out when
the ship reaches the first point, and is flown. Then a Tab queue is ended by the bar's
Stop (`A`), and another by a plain right-click. Last, a queue for `ship1` and `ship3`
gets a point with only `ship3` selected: that ship goes on in a copy of the queue and the
other keeps the old one.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "QOL.log" contains "move queues: Tab points (and minimap points) queued per ship, flown at once, patched".
5. Scene "select ship2".
6. Expect scene "selection" answers "ship2".
7. Move the mouse to 840,360.
8. Scene "waypoint-key on".
9. Right-click at 840,360.
10. Wait up to 10 seconds for "QOL.log" to say "move queue: run sent: go, 1 point(s) from point 1, 1 ship(s)".
11. Wait 2 seconds.
12. Right-click at 840,200.
13. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 2 (go)".
14. Wait 1 seconds.
15. Right-click at 520,200.
16. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 3 (go)".
17. Scene "waypoint-key off".
18. Wait up to 10 seconds for "QOL.log" to say "move queue: key let go, the points are queued already".
19. Wait up to 60 seconds for "QOL.log" to say "done, 1 ship(s)".
20. Take a screenshot called "queue-flown".
21. Scene "waypoint-key on".
22. Right-click at 640,300.
23. Wait up to 10 seconds for "QOL.log" to say "move queue: run sent: go, 1 point(s) from point 1, 1 ship(s)".
24. Right-click at 180,530.
25. Wait up to 10 seconds for "UI.log" to say "queued (QOL)".
26. Expect "QOL.log" contains "move queue: appended point 2 (warp)".
27. Scene "waypoint-key off".
28. Wait up to 60 seconds for "QOL.log" to say "move queue: next run sent: warp, 1 point(s) from point 2, 1 ship(s)".
29. Wait up to 90 seconds for "QOL.log" to say "done, 1 ship(s)".
30. Take a screenshot called "warped".
31. Press Home.
32. Scene "select ship1".
33. Scene "waypoint-key on".
34. Right-click at 900,200.
35. Right-click at 900,450.
36. Wait up to 10 seconds for "QOL.log" to say "move queue: run sent: go, 2 point(s) from point 1, 1 ship(s)".
37. Scene "waypoint-key off".
38. Wait 2 seconds.
39. Click at 249,640.
40. Wait up to 10 seconds for "QOL.log" to say "move queue: cleared for 1 ship(s): stop".
41. Scene "waypoint-key on".
42. Right-click at 1000,250.
43. Right-click at 1000,450.
44. Wait up to 10 seconds for "QOL.log" to say "move queue: run sent: go, 2 point(s) from point 1, 1 ship(s)".
45. Scene "waypoint-key off".
46. Wait 2 seconds.
47. Right-click at 640,300.
48. Wait up to 10 seconds for "QOL.log" to say "move queue: cleared for 1 ship(s): move".
49. Scene "select ship1 ship3".
50. Scene "waypoint-key on".
51. Right-click at 640,150.
52. Right-click at 400,150.
53. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 2 (go)".
54. Scene "waypoint-key off".
55. Scene "select ship3".
56. Scene "waypoint-key on".
57. Right-click at 400,400.
58. Wait up to 10 seconds for "QOL.log" to say "(split from queue".
59. Scene "waypoint-key off".
60. Expect "QOL.log" contains "move queue: appended point 3 (go)".
61. Wait up to 60 seconds for "QOL.log" to say "done, 1 ship(s)".
62. Expect the game is still running.
63. Quit the game.
64. Expect no crash.
