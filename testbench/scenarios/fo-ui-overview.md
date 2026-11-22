# Fleet Operations: Roots with the UI: the move ring, the height posts and the placement footprint are ours

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=0 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=1 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (FO-6 step 4, `PLAN-FLEETOPS.md`, "Step 4: the panels
Roots changes least"). The match is `_fo-render3d-skirmish`'s: Federation against Klingon,
Instant Action, `first`. `mRenderWhere`'s post (+0x9d) is FleetOpsHook's; the two DrawQuad calls the order ring takes
(0x5222f0, `mRenderGridFX`'s 0x522537) are stock's. The drag box stays the engine's (decision 74).

The drag selects the first Mediterranean class ship alone (as `fo-ui-bar` does): the start fleet's two
construction ships stand at about 420,600 and 515,680 on the 1280x960 screen, and a box reaching to x=520 took the second
only now and then, which changed the bar (a mixed selection has no build button at 600,788) and put the
click on the starbase's tooltip. The height posts are not judged: on Roots the pointed unit's post is
FleetOpsHook's own edit of `mNormalModeRender`, the move point's is its `mRenderWhere`, and a selected
ship has none in stock either (decision 180).

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Move the mouse to 700,300.
4. Wait 1 second.
5. Drag from 375,560 to 465,650.
6. Wait 3 seconds.
7. Right-click at 640,480.
8. Wait 1 second.
9. Take a screenshot called "move ring".
10. Check that a ring or marker is drawn on the map at the point that was clicked, and that a dotted move line runs from the selected ship to it.
11. Expect "UI.log" contains "leftovers:".
12. Move the mouse to 600,788.
13. Wait 1 second.
14. Move the mouse to 604,790.
15. Wait 1 second.
16. Click at 604,790.
17. Wait 2 seconds.
18. Take a screenshot called "build list".
19. Move the mouse to 318,788.
20. Wait 1 second.
21. Move the mouse to 322,790.
22. Wait 1 second.
23. Click at 322,790.
24. Wait 2 seconds.
25. Take a screenshot called "placing".
26. Move the mouse to 700,420.
27. Wait 1 second.
28. Move the mouse to 704,424.
29. Wait 2 seconds.
30. Expect "UI.log" contains "placement: footprint".
31. Take a screenshot called "placement".
32. Check that a square footprint of the building about to be placed is drawn on the map at the pointer, near 704,424: translucent red with diagonal hatching and small crossed cells, because the starbase beside it is in the way, and that a grid of amber tiles (the cells the starbase takes, which is ours too and not a second marker) lies under and round the starbase. Corner brackets are not required.
33. Press Escape.
34. Expect "UI.log" not contain "PANIC".
35. Expect "Display8.log" not contain "VUID".
36. Expect "Display8.log" not contain "device removed".
37. Quit the game.
38. Expect the game to have exited.
39. Expect no crash.
