# Fleet Operations: Roots with QOL: Tab points queue per ship and are flown, beside the engine's own move paths

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d qol
Setup: display/bench-mode.sh stub Backend=vulkan testbench/bench/roots-waypoint.sh
Timeout: 16 min

FO-7 round three (`PLAN-FLEETOPS.md`, "FO-7: QOL on Roots", decision 114). `qol.move_paths` is
verified on this build: every function QOL reads or hooks for the queue (`mNormalModeRightClick`,
`mProcessWaypoints`, `mAddPathPoint`, `GameObject::QueueCommand`, the `NetOrder*::Execute` family,
`FormationManager`) is stock's byte for byte in `ArmadaL.exe` and in memory after FleetOpsHook,
and Roots' edits to `cOverViewImp::Simulate` and `mProcessKeyboardInput` lie elsewhere in them.
`Craft::Say` is replaced whole by FleetOpsHook, so QOL's quiet sends cannot work here and the
ships speak as they take a leg (the bench has no sound; the log does not carry it either).

The bench cannot hold Tab (`testbench/README.md`, "A key cannot be held"), so the Setup's
`roots-waypoint.sh` binds Roots' `enter_waypoint` to the middle mouse button in the clone's
`Input.map`, which a scenario can hold. The button's own camera rotation moves the view if the
pointer moves while it is held, so each point is one step that goes there, holds the button for the
right-click and lets it go. A point is a queue point whatever the gaps between holds.

An Instant Action game as Federation (`_fo-sides`); the player has two Mediterranean class
construction ships at about 335,640 and 465,760 (the bench's start of every Roots scenario), and
a drag over both selects them. First three points are queued: the ships get a new queue with
a place each from Roots' formation (`spread`, formation 3), a path order each, and fly to the
end (`done`). Then a second queue is started and a plain right-click, with no key, sends the
ships elsewhere before they arrive: QOL clears the queue (`cleared ... move`) and, with render3d's
`EngineMoves = all`, the plain move is drawn as the engine's path (`(engine)`) while QOL's queue
is drawn as `(qol)`: QOL's queue takes precedence only where a ship has one.

1. Launch the game.
2. Wait for the main menu.
3. Expect "QOL.log" contains "build armada2/fo-roots-5.0Z11".
4. Expect "QOL.log" contains "PathStartsAtOnce=1  -> move queues: Tab points (and minimap points) queued per ship, flown at once, patched".
5. Expect "QOL.log" contains "WarpDistance=0  (0: moves left alone)".
6. Expect "QOL.log" not contain "patch refused".
7. Expect "QOL.log" not contain "qol.move_paths stands down".
8. Include "_fo-sides".
9. Click "LAUNCH".
10. Wait up to 180 seconds for "6000".
11. Wait 5 seconds.
12. Expect "Render3D.log" contains "overlays: move paths: QOL's queue found".
13. Take a screenshot called "start".
14. Note "the two Mediterranean class ships are selected; three points are queued with the middle button held for each right-click".
15. Drag from 290,590 to 520,800.
16. Wait 1 second.
17. Right-click at 560,330 holding the middle mouse button.
18. Wait up to 10 seconds for "QOL.log" to say "move queue: run sent: go, 1 point(s) from point 1, 2 ship(s)".
19. Right-click at 600,150 holding the middle mouse button.
20. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 2 (go)".
21. Right-click at 400,300 holding the middle mouse button.
22. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 3 (go)".
23. Expect "QOL.log" contains "move queue: new queue 1, ships 2".
24. Expect "QOL.log" contains "spread (new queue)".
25. Wait up to 10 seconds for "Render3D.log" to say "point(s) ahead (qol)".
26. Take a screenshot called "queued".
27. Check that a fine dotted line runs from each of the two selected ships through the queued points, which are marked by small dots, and that the two ships' lines end at different places (each ship has its own place in the group).
28. Wait up to 120 seconds for "QOL.log" to say "move queue: queue 1 done, 2 ship(s)".
29. Wait 1 second.
30. Take a screenshot called "flown".
31. Check that the two ships have stopped side by side and no dotted line is left.
32. Note "a second queue, then a plain right-click ends it, and the engine's own path is drawn for the plain move".
33. Right-click at 500,200 holding the middle mouse button.
34. Right-click at 300,150 holding the middle mouse button.
35. Wait up to 10 seconds for "QOL.log" to say "move queue: appended point 2 (go), queue 2".
36. Move the mouse to 700,120.
37. Wait 1 second.
38. Right-click at 700,120.
39. Wait up to 10 seconds for "QOL.log" to say "move queue: cleared for 2 ship(s): move".
40. Wait up to 20 seconds for "Render3D.log" to say "point(s) ahead (engine)".
41. Take a screenshot called "plain move".
42. Check that a fine dotted line runs from each of the two selected ships to the clicked point at the top of the screen (the engine's path of a plain move, drawn because every engine move is), and none runs to the queue's old points.
43. Wait up to 120 seconds for "Render3D.log" to say "0 selected ship(s) on a path".
44. Expect "QOL.log" not contain "patch refused".
45. Expect "Display8.log" not contain "VUID".
46. Expect "Display8.log" not contain "device removed".
47. Quit the game.
48. Expect the game to have exited.
49. Expect no crash.
