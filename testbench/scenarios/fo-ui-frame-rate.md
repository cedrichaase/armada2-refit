# Fleet Operations: Roots with the UI: the frame-rate panel is ours (FRAMERATE_DISPLAY in the clone's RTS_CFG.h)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh --rts FRAMERATE_DISPLAY=1 HUD.Enable=1 HUD.Resources=0 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=1 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (FO-6 step 4, `PLAN-FLEETOPS.md`, "Step 4: the panels
Roots changes least"). The match is `_fo-render3d-skirmish`'s: Federation against Klingon,
Instant Action, `first`. `Armada_RenderFrame`'s edit (+0x245) is FleetOpsHook's, away from the two print calls the hook takes
(+0x286, +0x290): verified in the file and in memory.

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Take a screenshot called "frame rate".
4. Check that a dark panel at the top left lists FPS, frame and render times in the UI's style.
5. Expect "UI.log" contains "leftovers: 1 of the engine's last draws are ours".
6. Expect "UI.log" contains "leftovers: frame rate".
7. Expect "UI.log" not contain "PANIC".
8. Expect "Display8.log" not contain "VUID".
9. Expect "Display8.log" not contain "device removed".
10. Quit the game.
11. Expect the game to have exited.
12. Expect no crash.
