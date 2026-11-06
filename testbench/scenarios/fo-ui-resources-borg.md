# Fleet Operations: Roots with the UI: the resources of the Borg are named as Roots names them

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=1 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 10 min

FO-6 step 4a (`PLAN-FLEETOPS.md`, decision 153): the player is the Borg (the player's side list
at its row 134, `fo-render3d-races`) against the Federation AI; the cells' names are read from the
tooltips Roots' own string table answers for that race, so the log's "resources:" line names
them.

1. Launch the game.
2. Wait for the main menu.
3. Move the mouse to 640,520.
4. Wait 1 second.
5. Click "Instant Action".
6. Wait up to 30 seconds for "LAUNCH".
7. Wait 3 seconds.
8. Move the mouse to shell 270,114.
9. Wait 1 second.
10. Click at shell 278,114.
11. Wait 2 seconds.
12. Move the mouse to shell 278,134.
13. Wait 1 second.
14. Click at shell 278,134.
15. Wait 2 seconds.
16. Expect "Borg" is visible.
17. Click "LAUNCH".
18. Wait up to 240 seconds for "UI.log" to say "resources:".
19. Wait 3 seconds.
20. Take a screenshot called "resources".
21. Check that the resource strip at the top left names each resource in words and shows an amount.
22. Expect "UI.log" not contain "PANIC".
23. Expect "Display8.log" not contain "VUID".
24. Expect "Display8.log" not contain "device removed".
25. Quit the game.
26. Expect the game to have exited.
27. Expect no crash.
