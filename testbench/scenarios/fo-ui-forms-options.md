# Fleet Operations: Roots with the UI: the in-mission menu and Yes/No are ours, over FleetOpsHook's forms

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1
Timeout: 16 min

Run with `a2test --game fo-roots` and `--install .` (`PLAN-FLEETOPS.md`, FO-6 step 7,
option C; `ui/MENUS.md`, "Roots' forms"). Esc in a skirmish opens Roots' Options form in
its mission mode (Back bound to the return to the game, End Mission and Exit to Windows
shown); the UI draws it as ours over the mission's last frame. End Mission asks Roots'
Yes/No form, drawn as ours with the question from the form's own picture; No closes it and
the return button ends the menu, the skirmish going on.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Include "_fo-sides".
5. Click "LAUNCH".
6. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
7. Wait 8 seconds.
8. Move the mouse to 700,300.
9. Press Escape.
10. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
11. Wait 3 seconds.
12. Take a screenshot called "mission options".
13. Check that a window of ours titled "OPTIONS" stands over the skirmish (its HUD and map behind it), with a column of buttons (among them End Mission and Exit to Desktop) and Return to Game below them.
14. Click "End Mission".
15. Wait up to 20 seconds for "UI.log" to say "forms: TYesNoForm is ours".
16. Wait 3 seconds.
17. Take a screenshot called "yes no".
18. Check that a small panel of ours asks whether to end the mission, with two buttons of ours, over the options window.
19. Click "No".
20. Wait up to 20 seconds for "UI.log" to say "closed after".
21. Wait 3 seconds.
22. Take a screenshot called "back to options".
23. Click "Return".
24. Wait up to 20 seconds for "UI.log" to say "closed after".
25. Wait 4 seconds.
26. Take a screenshot called "mission".
27. Check that the skirmish is shown again with no menu over it.
28. Expect "UI.log" contains "over the mission's last frame".
29. Expect "UI.log" not contain "PANIC".
30. Expect "Display8.log" not contain "VUID".
31. Stop the game.
32. Expect no crash.
