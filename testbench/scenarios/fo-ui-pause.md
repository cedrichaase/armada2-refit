# Fleet Operations: Roots with the UI: the pause box is ours, with words of its own where Roots' string table gives none

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=0 HUD.Buttons=0 HUD.Pause=1 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (FO-6 step 4, `PLAN-FLEETOPS.md`, "Step 4: the panels
Roots changes least"). The match is `_fo-render3d-skirmish`'s: Federation against Klingon,
Instant Action, `first`. Roots' string table answers `GUI_POP_PAUSE_TEXT` and `GUI_POP_PAUSE_CONTINUE` with an empty string (the first run showed
a blank box), so the box falls back to the UI's own words. The pause key pauses; the box's button presses the
stock unpause button.

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Move the mouse to 700,300.
4. Press Pause.
5. Wait up to 15 seconds for "UI.log" to say "pause: shown".
6. Wait 2 seconds.
7. Take a screenshot called "paused".
8. Check that the screen is dimmed and a small panel in the middle reads GAME PAUSED with one button under it reading CONTINUE.
9. Expect "UI.log" contains "panels: PauseBox ours".
10. Click at 647,513.
11. Wait up to 10 seconds for "UI.log" to say "panels: click Continue".
12. Wait 3 seconds.
13. Take a screenshot called "continued".
14. Check that the dimming and the pause panel are gone and the match is shown.
15. Expect "UI.log" not contain "PANIC".
16. Expect "Display8.log" not contain "VUID".
17. Expect "Display8.log" not contain "device removed".
18. Quit the game.
19. Expect the game to have exited.
20. Expect no crash.
