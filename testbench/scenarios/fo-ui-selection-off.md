# Fleet Operations: Roots with the UI on and the selection overlay Roots' own (the A/B of fo-ui-selection)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=0 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (FO-6 step 4, `PLAN-FLEETOPS.md`, "Step 4: the panels
Roots changes least"). The match is `_fo-render3d-skirmish`'s: Federation against Klingon,
Instant Action, `first`. The same box selection with nothing of the HUD taken, for the judge to set beside `fo-ui-selection`'s shot.

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Move the mouse to 700,300.
4. Wait 1 second.
5. Drag from 30,120 to 1250,640.
6. Wait 3 seconds.
7. Take a screenshot called "selected, Roots'".
8. Expect "UI.log" not contain "PANIC".
9. Expect "Display8.log" not contain "VUID".
10. Expect "Display8.log" not contain "device removed".
11. Quit the game.
12. Expect the game to have exited.
13. Expect no crash.
