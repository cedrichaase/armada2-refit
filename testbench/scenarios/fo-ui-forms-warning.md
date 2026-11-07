# Fleet Operations: Roots with the UI: the Warning prompt (Exit to Desktop, End Mission) is a dialog of ours

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1
Timeout: 16 min

Run with `a2test --game fo-roots` and `--install .` (`ui/MENUS.md`, "Roots' forms"). Esc in a skirmish opens
Roots' Options form in its mission mode; Exit to Desktop and End Mission each ask Roots'
Warning form (`TYesNoForm`), which the UI draws as a dialog of its own (a themed panel, the
question in our font, the form's icon smaller, Yes and No); No closes it, Esc answers No.

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
21. Click "Exit to Desktop".
22. Wait up to 20 seconds for "UI.log" to say "forms: TYesNoForm is ours".
23. Wait 3 seconds.
24. Take a screenshot called "warning exit".
25. Check that a dialog of ours (a themed panel, sized to its text and centred) shows the title "Warning", a small warning triangle, the whole question "Are you sure you want to exit to Windows?" in our font, wrapped inside the panel with nothing cut, and Yes and No once each at its foot.
26. Click "No".
27. Wait up to 20 seconds for "UI.log" to say "closed after".
28. Wait 3 seconds.
29. Click "End Mission".
30. Wait up to 20 seconds for "UI.log" to say "forms: TYesNoForm is ours".
31. Wait 3 seconds.
32. Take a screenshot called "warning end mission".
33. Check that the same dialog shows the End Mission question whole, with Yes and No once each.
34. Press Escape.
35. Wait up to 20 seconds for "UI.log" to say "closed after".
36. Wait 3 seconds.
37. Take a screenshot called "escape answered no".
38. Check that the Warning dialog is gone and the Options window of ours is shown again, the skirmish going on behind it.
39. Click "Return".
40. Wait up to 20 seconds for "UI.log" to say "closed after".
41. Wait 4 seconds.
42. Take a screenshot called "mission"
43. Check that the skirmish is shown again with no menu over it.
44. Expect "UI.log" not contain "PANIC".
45. Expect "Display8.log" not contain "VUID".
46. Stop the game.
47. Expect no crash.
