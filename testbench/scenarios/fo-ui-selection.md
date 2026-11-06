# Fleet Operations: Roots with the UI: the selection overlay is ours (brackets and gauges), Roots' own HUD kept

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=0 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=0 HUD.Selection=1 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (FO-6 step 4, `PLAN-FLEETOPS.md`, "Step 4: the panels
Roots changes least"). The match is `_fo-render3d-skirmish`'s: Federation against Klingon,
Instant Action, `first`. The selection overlay (`hud.reads_overlay`): `SelectionDisplay`'s PostLoad, Render and Cleanup are FleetOpsHook's
trampolines; ours reads the selection list and the units' gauges and draws brackets and bars, and the panel
switch skips the stock component's Render. A box selection of the start fleet.

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Move the mouse to 700,300.
4. Wait 1 second.
5. Drag from 30,120 to 1250,640.
6. Wait 3 seconds.
7. Take a screenshot called "selected, ours".
8. Check that the selected station and ships are marked by thin yellow corner brackets with a green and blue gauge above each, and that Fleet Operations' own HUD (minimap, info strip, button bar) is still drawn as before.
9. Expect "UI.log" contains "selection: ours".
10. Expect "UI.log" contains "selection: the 3D bubbles off".
11. Move the mouse to 884,788.
12. Wait 2 seconds.
13. Take a screenshot called "bar hover".
14. Move the mouse to 840,380.
15. Wait 3 seconds.
16. Take a screenshot called "unit tooltip".
17. Check that Fleet Operations' own tooltip for the pointed unit (its name, hitpoints and what it produces) is drawn beside the pointer, and the selection brackets are still drawn.
18. Expect "UI.log" not contain "PANIC".
19. Expect "Display8.log" not contain "VUID".
20. Expect "Display8.log" not contain "device removed".
21. Quit the game.
22. Expect the game to have exited.
23. Expect no crash.
