# Fleet Operations: Roots with the UI: the mission's name at its start, ours, clear of the resource strip

Resolutions: 2150x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.BarLabels=1 HUD.InfoCamera=1 HUD.Overlay=1 HUD.LayoutLog=1
Timeout: 14 min

The user's report of 2026-11-07 (Roots 5.0Z11, 3440x1440): at the start of the first allied
tutorial the mission's name ("Gameplay Tutorial", blue) shows for a few seconds at the very
top left, under and beside the resource strip of ours. It is a message of the engine's
`OverlayText` (the script's `AddMessage` at the panel's corner), Roots' own until `[HUD]
Overlay=1` (PLAN-FLEETOPS.md, decisions 326 onward). Ours draws it in its colour and with the engine's fade, and a message that would touch a
panel of ours is centred across the screen at its height (here the top centre). The way in is `fo-ui-briefing`'s; the shots run from the load's end.

1. Launch the game.
2. Wait for the main menu.
3. Wait 5 seconds.
4. Move the mouse to 1075,361.
5. Wait 1 second.
6. Click at 1076,363.
7. Wait up to 15 seconds for "Previous Menu".
8. Wait 4 seconds.
9. Move the mouse to 235,225.
10. Wait 1 second.
11. Click at 237,225.
12. Wait up to 20 seconds for "TUTORIAL: Gameplay".
13. Wait 2 seconds.
14. Move the mouse to 640,233.
15. Wait 1 second.
16. Click "TUTORIAL: Gameplay".
17. Wait 1 second.
18. Double-click "TUTORIAL: Gameplay".
19. Wait 2 seconds.
20. Move the mouse to 540,780.
21. Wait 1 second.
22. Take a screenshot called "mission selected".
23. Click "OK".
24. Wait up to 180 seconds for "UI.log" to say "overlay: #".
25. Take 8 screenshots called "start" every 1 seconds.
26. Check that the series "start" shows the mission's name ("Gameplay Tutorial") in blue at the top centre of the screen, clear of the resource strip at the top left and of the round buttons at the top right, with no second copy of it drawn by the game.
27. Expect "UI.log" contains "panels: OverlayText ours".
28. Expect "UI.log" contains "Gameplay Tutorial".
29. Expect "UI.log" contains "overlay: #0 moved clear of the panels of ours".
30. Wait 10 seconds.
31. Take a screenshot called "later".
32. Check that the mission's name is gone (it shows only at the mission's start).
33. Expect "UI.log" not contain "PANIC".
34. Expect "Display8.log" not contain "VUID".
35. Expect "Display8.log" not contain "device removed".
36. Quit the game.
37. Expect the game to have exited.
38. Expect no crash.
