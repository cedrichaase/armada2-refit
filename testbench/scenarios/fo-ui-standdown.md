# Fleet Operations: Roots with the UI and every part of it at 0: Roots as Display8 and Render3D alone give it

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Loading.Enable=0 UI.RefitKey=0
Timeout: 12 min

The B side of `fo-ui-standdown-ref`, which runs first and records the baseline
`fo-ui-standdown` (FO-6 step 3, `PLAN-FLEETOPS.md`). `UI.asi` is installed with Roots'
defaults (`ui/defaults/fo-roots.ini`: every HUD panel, the menus, the Credits, the menu
scene, the cursor and the movies at 0) and the two parts Roots has on switched off too: the
loading screen (`Loading.Enable=0`) and the Refit screen's key (`UI.RefitKey=0`). The hooks
that need no key stay in (the mission frame, input, the loading overlay) and draw nothing:
the main menu and the setup screen must match the reference pixel for pixel, as near as two
runs of the shell agree (the pointer, which the screenshot catches where the session left it, is the only difference seen: about 100 pixels), and the match must look as the reference's. Run with
`a2test --game fo-roots` and `--install .`.

1. Launch the game.
2. Wait for the main menu.
3. Wait 5 seconds.
4. Take a screenshot called "menu".
5. Include "_fo-sides".
6. Take a screenshot called "setup".
7. Expect the screenshot "menu" matches baseline "fo-ui-standdown" with at most 0.05% of pixels over 4.
8. Expect the screenshot "setup" matches baseline "fo-ui-standdown" with at most 0.05% of pixels over 4.
9. Click "LAUNCH".
10. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
11. Wait 15 seconds.
12. Take a screenshot called "match".
13. Check that the match has started: the 3D view of a space map with ships and Fleet Operations' own HUD (its resource counters, minimap and info strip), and nothing of the UI's over it (no panel, button or window of a different style).
14. Wait up to 20 seconds for the logs the next steps expect.
15. Expect "UI.log" contains "engine: build armada2/fo-roots-5.0Z11".
16. Expect "UI.log" contains "mission: hooked".
17. Expect "UI.log" contains "hud: stock".
18. Expect "UI.log" contains "UI frames in".
19. Expect "UI.log" not contain "a screen of ours".
20. Expect "UI.log" not contain "first frame of ours".
21. Expect "UI.log" not contain "refit: opened".
22. Expect "UI.log" not contain "menus: hosted".
23. Expect "UI.log" not contain "PANIC".
24. Expect "Display8.log" not contain "VUID".
25. Expect "Display8.log" not contain "device removed".
26. Quit the game.
27. Expect the game to have exited.
28. Expect no crash.
