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

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Move the mouse to 700,300.
4. Wait 1 second.
5. Drag from 300,560 to 520,745.
6. Wait 3 seconds.
7. Right-click at 640,480.
8. Wait 1 second.
9. Take a screenshot called "move ring".
10. Check that a ring or marker is drawn on the map at the point that was clicked, and the selected ships have height posts.
11. Expect "UI.log" contains "leftovers:".
12. Click at 600,788.
13. Wait 2 seconds.
14. Click at 322,790.
15. Wait 1 second.
16. Move the mouse to 700,420.
17. Wait 1 second.
18. Move the mouse to 704,424.
19. Wait 2 seconds.
20. Take a screenshot called "placement".
21. Check that a footprint of the building about to be placed follows the pointer on the map (a green or red marker with corner brackets) and that no Fleet Operations marker is drawn a second time under it.
22. Press Escape.
23. Expect "UI.log" not contain "PANIC".
24. Expect "Display8.log" not contain "VUID".
25. Expect "Display8.log" not contain "device removed".
26. Quit the game.
27. Expect the game to have exited.
28. Expect no crash.
