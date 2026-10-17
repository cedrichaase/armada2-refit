# The HUD's panels keep clear of each other and of the 3D view, at every aspect and selection

Resolutions: 1024x768, 1920x1080, 2560x1080
Mod: refit
Launch: -nointro a2_borg01
Setup: ui/bench-asi.sh --no-grid --scene ui-hud-layout UI.Theme=federation HUD.LayoutLog=1
Sheet: 1 ship, 6 ships, 30 ships
Timeout: 10 min

The UI's HUD (`ui/HUD.md`) laid out at 4:3, 16:9 and 21:9 with one, six and thirty
ships selected. In 16:9 the bar once overlapped the info panel, and with about thirty
ships selected the info panel grew so tall that it covered the ships in the view; only
promo footage showed it. The checks read the panels' rectangles from UI.asi itself:
with `[HUD] LayoutLog=1` it writes each new layout to UI.log ("layout: 1920x1080 |
minimap x,y,w,h | info ... | bar ..."; `ui/HUD.md`, "The layout log"), so nothing here
is guessed from pixels. The scene (`testbench/scene/scenes/ui-hud-layout.ini`) is
thirty Federation destroyers, `s1`..`s30`, six by five about the camera's start.
`Theme=federation` fixes the look: its LCARS frames take the most room, and the race
of a direct `a2_borg01` launch is dealt at random.

The thresholds, per layout:

| Check | Fails when |
|---|---|
| overlap | two panels share more than a pixel on both axes |
| height | a panel standing on the bottom half of the screen (minimap, info panel, bar, unit view) has its top edge more than 45% of the screen's height up from the bottom (60% at 4:3): the middle of the 3D view, where the ships are, is to stay clear |
| cover | all the panels together (their union) cover more than 30% of the screen (42% at 4:3) |

4:3 gets looser limits (the user's call, 2026-10-17: hardly anyone plays at 4:3 any more):
there the HUD's fixed-size panels take a larger share of a narrower screen. A step's
`(N% at 4:3)` applies to any screen narrower than 3:2.

The run's `ui-hud-layout/sheet.png` has the screenshots as a matrix, a row per
resolution and a column per selection, with the panels drawn on them (red where a check
failed).

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "UI.log" contains "info: ours".
5. Expect "UI.log" contains "bar: ours".
6. Move the mouse to hud 400,150.
7. Scene "select s15".
8. Wait 3 seconds.
9. Expect scene "selection" answers "selected 1:".
10. Take a layout screenshot called "1 ship".
11. Expect the HUD's panels not to overlap.
12. Expect the HUD's panels at the bottom to rise at most 45% of the screen's height (60% at 4:3).
13. Expect the HUD's panels to cover at most 30% of the screen (42% at 4:3).
14. Scene "select s13 s14 s15 s16 s17 s18".
15. Wait 3 seconds.
16. Expect scene "selection" answers "selected 6:".
17. Take a layout screenshot called "6 ships".
18. Expect the HUD's panels not to overlap.
19. Expect the HUD's panels at the bottom to rise at most 45% of the screen's height (60% at 4:3).
20. Expect the HUD's panels to cover at most 30% of the screen (42% at 4:3).
21. Scene "select s1 s2 s3 s4 s5 s6 s7 s8 s9 s10 s11 s12 s13 s14 s15 s16 s17 s18 s19 s20 s21 s22 s23 s24 s25 s26 s27 s28 s29 s30".
22. Wait 3 seconds.
23. Expect scene "selection" answers "selected 30:".
24. Take a layout screenshot called "30 ships".
25. Expect the HUD's panels not to overlap.
26. Expect the HUD's panels at the bottom to rise at most 45% of the screen's height (60% at 4:3).
27. Expect the HUD's panels to cover at most 30% of the screen (42% at 4:3).
28. Expect the game is still running.
29. Expect no crash.
