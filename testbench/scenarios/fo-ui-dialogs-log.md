# Fleet Operations: Roots with the UI: the Admiral's Log as a screen of ours

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Scene=1 Menus.Enable=1 Menus.Credits=1
Timeout: 12 min

Run with `a2test --game fo-roots` and `--install .`. Roots' Admiral's Log
(`AdmiralsLogDlgProc`, opened from the in-mission menu) is adopted and drawn by its spec of
ours (PLAN-FLEETOPS.md, decision 259; ui/MENUS.md, "Roots' dialogs, laid out"): Armada
II's log window, its tabs, players and Done the stock owner-drawn buttons, the page the
shown pane's own picture (Roots' panes are pop-ups whose labels hold no text). A tab
switches the page; Done closes the log. A skirmish is started from Instant Action
(`fo-ui-dialogs-saves`).

1. Launch the game.
2. Wait up to 180 seconds for "UI.log" to say "forms: TMainForm is ours".
3. Wait 3 seconds.
4. Move the mouse to 700,300.
5. Wait 1 second.
6. Click "Instant Action".
7. Wait up to 30 seconds for "UI.log" to say "game setup drawn by its spec".
8. Wait 3 seconds.
9. Move the mouse to 1225,852.
10. Wait 1 second.
11. Click "LAUNCH".
12. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
13. Wait 10 seconds.
14. Move the mouse to 700,300.
15. Press Escape.
16. Wait up to 20 seconds for "UI.log" to say "TOptionsMenuForm is ours".
17. Wait 2 seconds.
18. Click "View Admiral's Log".
19. Wait up to 20 seconds for "UI.log" to say "admiral's log drawn by its spec".
20. Wait 4 seconds.
21. Take a screenshot called "log".
22. Expect the window manager to see one window.
23. Check that a window of ours titled "Admiral's Log" shows a row of tabs (Score, Military, Economy, Timeline, Battles, Ships, Build, Tally), the two players (steamuser and an AI, each with its team) down the left, a page with the score table to their right, and "Done" at the foot.
24. Click "Timeline".
25. Wait 3 seconds.
26. Take a screenshot called "timeline".
27. Check that the page now shows the timeline (a graph of the players over time), not the score table of the earlier screenshot, and the Timeline tab is the selected one.
28. Click "Done".
29. Wait up to 20 seconds for "UI.log" to say "released".
30. Wait 2 seconds.
31. Take a screenshot called "closed".
32. Expect "UI.log" not contain "PANIC".
33. Expect "Display8.log" not contain "VUID".
34. Stop the game.
35. Expect no crash.
