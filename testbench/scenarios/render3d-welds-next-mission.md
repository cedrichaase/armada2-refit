# Welds of one mission, then the next mission: no crash, no flip to windowed

Resolutions: 16:9
Mod: refit
Setup: testbench/scene/bench-setup.sh shields
Timeout: 12 min

A player aborted a mission while ships were being repaired and started an Instant Action
match: at about 90% of the loading the game crashed, the engine's exception handler made its
display again windowed on a new window (Display8: "CreateDevice on another window ... the
swap chain follows it") and then hung in its call-stack dump, a black screen
(`render3d/README.md`, "effects", Repair). render3d's welds of the old mission were still
listed at the new mission's first frame, and the welds' light (`lights()`, before `draw()`
drops the welds whose particle is gone) placed them: the old handle named an object of the
new mission, the old hardpoint node was freed memory, and `Entity::GetWorldPosition` faulted.

An Instant Action match on Warzone is started from the main menu, and the `shields` scene
built in it, with the target's healing off, gets welds going (as `shields-down-repair`);
the match is aborted while they burn. The second match has two more computer players, whose
starting units take the handles the scene's ships had, so the welds' handle names an
object again at the new match's first view. There the welds of the old match must be dropped before their light is placed (`Render3D.log`: "... weld(s) of a view N s
ago dropped before their light"), and the match must come up and draw, with no new device
on another window. Before the fix the stale welds were placed; whether that faulted
depended on what the freed hardpoint node held, so the log line is the check, not a
crash.

1. Launch the game.
2. Wait for the main menu.
3. Click the instant action cube.
4. Wait for "GAME SETUP".
5. Click "LAUNCH".
6. Wait up to 120 seconds for "Scene.log" to say "scene ready".
7. Scene "heal target off".
8. Scene "orbit target 140 30 95".
9. Wait up to 60 seconds for "Render3D.log" to say "shields down on".
10. Scene "weapons shooter off".
11. Scene "weapons klingon off".
12. Wait up to 60 seconds for the logs the next steps expect.
13. Expect "Render3D.log" contains "effects: weld on fgalaxy".
14. Expect "Render3D.log" contains "effects: first welds drawn".
15. Take a screenshot called "welds".
16. Press Escape.
17. Wait 2 seconds.
18. Click "Abort Mission".
19. Wait 2 seconds.
20. Click "Yes".
21. Wait up to 20 seconds for "Done".
22. Click "Done".
23. Wait for the main menu.
24. Click the instant action cube.
25. Wait for "GAME SETUP".
26. Click "Empty Slot".
27. Wait 1 second.
28. Click "Computer AI:Easy".
29. Wait 1 second.
30. Click "Empty Slot".
31. Wait 1 second.
32. Click "Computer AI:Easy".
33. Wait 1 second.
34. Take a screenshot called "four players".
35. Click "LAUNCH".
36. Wait up to 90 seconds for "Render3D.log" to say "dropped before their light".
37. Wait up to 30 seconds for "UI.log" to say "UI frames in".
38. Take a screenshot called "next mission".
39. Expect no black bars.
40. Expect the game is still running.
41. Expect "Display8.log" not contain "CreateDevice on another window".
42. Expect "Render3D.log" not contain "PANIC".
43. Quit the game.
44. Expect no crash.
