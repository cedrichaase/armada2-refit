# Move paths: a Tab path drawn until flown, the ships under way at the first point

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh warp
Timeout: 20 min

Issue #17. `render3d/README.md`, "overlays" (the move paths), and `qol/README.md`,
QOL-10. Run it on a checkout: `./a2test run render3d-move-paths --install .`. The
`warp` scene's middle destroyer (`ship2`, at the middle of the screen) is selected and
sent along three points sketched with Tab held: to the right, up, then left
(each a few hundred units, an ordinary move). The bench cannot hold a key (Wine under
Xwayland lets it go at once), so Tab is held with `Scene "waypoint-key on"`, which
points the overview at a key state of Scene.asi's own, and let go with `off`. With `[QOL] PathStartsAtOnce=1` the
path order goes out at the first right-click, so the ship is on its way while Tab is
still held (render3d logs it on a path before the second point), and Tab let go sends
nothing more. With `[Overlays] MovePaths=1` render3d draws the path as fine light-green
dots with a bigger dot at each point, from the ship on; the part flown is gone, and
nothing is drawn once the ship has arrived.

Judge the shots: "held" shows the three points and the dots between them, the ship
already off towards the first; "released" the same path, still drawn; "mid-flight"
the ship along it with no dots behind it; "arrived" no path at all. Never stock's
one-pixel green line with its boxes.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "overlays: move paths 1, stock's sketch line ours".
5. Expect "QOL.log" contains "move queues: Tab points (and minimap points) queued per ship, flown at once, patched".
6. Scene "select ship2".
7. Expect scene "selection" answers "ship2".
8. Move the mouse to 840,360.
9. Scene "waypoint-key on".
10. Right-click at 840,360.
11. Wait up to 10 seconds for "QOL.log" to say "move queue: run sent: go, 1 point(s) from point 1".
12. Wait up to 10 seconds for "Render3D.log" to say "move paths: 1 ship(s) on a path, 1 point(s) ahead, sketch 1".
13. Wait 1 seconds.
14. Right-click at 840,200.
15. Right-click at 520,200.
16. Wait up to 10 seconds for "Render3D.log" to say "sketch 3".
17. Expect "QOL.log" contains "move queue: appended point 3 (go)".
18. Take a screenshot called "held".
19. Scene "waypoint-key off".
20. Wait up to 10 seconds for "QOL.log" to say "move queue: key let go, the points are queued already".
21. Wait 1 seconds.
22. Take a screenshot called "released".
23. Wait 2 seconds.
24. Take a screenshot called "mid-flight".
25. Wait up to 30 seconds for "Render3D.log" to say "move paths: 0 ship(s) on a path, 0 point(s) ahead, sketch 0".
26. Wait 2 seconds.
27. Take a screenshot called "arrived".
28. Expect the game is still running.
29. Quit the game.
30. Expect no crash.
