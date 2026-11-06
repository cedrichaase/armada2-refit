# Fleet Operations: Roots with the UI: the top-right buttons are ours, press Roots' own buttons, and Roots' strip is not drawn under them

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=0 HUD.Buttons=1 HUD.Pause=0 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (FO-6 step 4, `PLAN-FLEETOPS.md`, "Step 4: the panels
Roots changes least"). The match is `_fo-render3d-skirmish`'s: Federation against Klingon,
Instant Action, `first`. The five buttons of Roots' ButtonPanel (idle, view, comm, objectives, and the "more" toggle behind which the menu sits)
are read by their press functions (`buttons: panel` logs each field's rectangle); ours draws menu, comm,
idle, view and objectives, as on stock, over Roots' own, which the panel switch hides.

1. Include "_fo-render3d-skirmish".
2. Wait up to 60 seconds for "UI.log" to say "buttons: menu comm idle view objectives(off)".
3. Wait 3 seconds.
4. Take a screenshot called "buttons ours".
5. Check that the top right corner shows one dark panel with a row of round icon buttons in the UI's own style (the rightmost dimmed), and that none of Fleet Operations' own square top-right buttons is drawn under or beside it.
6. Expect "UI.log" contains "panels: ButtonPanel ours".
7. Expect "UI.log" contains "buttons: panel menu=ok".
8. Move the mouse to 1200,46.
9. Wait 3 seconds.
10. Take a screenshot called "view hover".
11. Check that a tooltip box is drawn under the buttons with the text of the button under the pointer.
12. Click at 1156,46.
13. Wait up to 10 seconds for "UI.log" to say "panels: click idle".
14. Move the mouse to 700,400.
15. Click at 1200,46.
16. Wait up to 10 seconds for "UI.log" to say "panels: click view".
17. Wait 2 seconds.
18. Take a screenshot called "view pressed".
19. Click at 1200,46.
20. Wait up to 10 seconds for "UI.log" to say "panels: click view".
21. Expect "UI.log" not contain "PANIC".
22. Expect "Display8.log" not contain "VUID".
23. Expect "Display8.log" not contain "device removed".
24. Quit the game.
25. Expect the game to have exited.
26. Expect no crash.
