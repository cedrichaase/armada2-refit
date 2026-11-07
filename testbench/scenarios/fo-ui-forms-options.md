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
Yes/No form, which is drawn by us as our panel around the form's own picture (the "Warning"
title, the icon and the question, which Roots paints into the form's background; the picture
is Roots' art, so its text can run to the form's edge) under our two buttons; No closes it
and the return button ends the menu, the skirmish going on.

The sides are not picked (`_fo-sides` checks them by OCR in Roots' own setup dialog, whose
lost clicks failed this scenario at random; a click there is preceded by a move and a pause): nothing here depends on the race, since the
wait is on render3d's log, and each wait after a click reads only lines written since it.

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 2 seconds.
4. Move the mouse to 640,520.
5. Wait 1 second.
6. Click "Instant Action".
7. Wait up to 30 seconds for "LAUNCH".
8. Wait 3 seconds.
9. Move the mouse to 560,872.
10. Wait 1 second.
11. Move the mouse to 601,875.
12. Wait 2 seconds.
13. Click "LAUNCH".
14. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
15. Wait 8 seconds.
16. Move the mouse to 700,300.
17. Press Escape.
18. Wait up to 20 seconds for "UI.log" to say "forms: TOptionsMenuForm is ours".
19. Wait 3 seconds.
20. Take a screenshot called "mission options".
21. Check that a window of ours titled "OPTIONS" stands over the skirmish (its HUD and map behind it), with a column of buttons (among them End Mission and Exit to Desktop) and Return to Game below them.
22. Click "End Mission".
23. Wait up to 20 seconds for "UI.log" to say "forms: TYesNoForm is ours".
24. Wait 3 seconds.
25. Take a screenshot called "yes no".
26. Check that a panel of ours, over the Options window, frames a black picture with a "Warning" title, an exclamation icon and a question about ending the mission (its last words may be cut at the picture's right edge: Roots paints that text), with Yes and No as two small buttons of ours at its foot.
27. Click "No".
28. Wait up to 20 seconds for "UI.log" to say "closed after".
29. Wait 3 seconds.
30. Take a screenshot called "back to options".
31. Click "Return".
32. Wait up to 20 seconds for "UI.log" to say "closed after".
33. Wait 4 seconds.
34. Take a screenshot called "mission".
35. Check that the skirmish is shown again with no menu over it.
36. Expect "UI.log" contains "over the mission's last frame".
37. Expect "UI.log" not contain "PANIC".
38. Expect "Display8.log" not contain "VUID".
39. Stop the game.
40. Expect no crash.
