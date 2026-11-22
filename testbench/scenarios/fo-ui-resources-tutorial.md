# Fleet Operations: Roots with the UI: the resources of the Federation economy tutorial, named as Roots names them and clear of Roots' own HUD art

Resolutions: 2150x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=1 HUD.Resources=1 HUD.Buttons=0 HUD.Pause=0 HUD.Chat=0 HUD.Selection=0 HUD.FrameRate=0 HUD.Overview=0 HUD.LayoutLog=1
Timeout: 14 min

The user's report of 2026-11-06 (3440x1440, the Federation economy tutorial): the names
read Metal and Biomatter where Roots says Tritanium and Supply, and the strip of ours was
painted over HUD art FleetOpsHook draws. FO-6 step 4a (`PLAN-FLEETOPS.md`, decisions
153 onward): the names are read at run time from the tooltip Roots itself answers (its
string table), the set's names only a fallback; the strip sits where Roots' own
`ResourcePanel` is. The mission is reached through the single-player screen as in
`fo-render3d-tutorial`; run it at 2150x900 (the user's aspect, 2.39) and again with
`--res 1280x960`.

1. Launch the game.
2. Wait for the main menu.
3. Move the mouse to 1075,361.
4. Wait 1 second.
5. Click "Single Player".
6. Wait 4 seconds.
7. Move the mouse to 235,225.
8. Wait 1 second.
9. Click at 237,225.
10. Wait up to 20 seconds for "TUTORIAL: Gameplay".
11. Wait 2 seconds.
12. Move the mouse to 640,233.
13. Wait 1 second.
14. Click "TUTORIAL: Gameplay".
15. Wait 1 second.
16. Double-click "TUTORIAL: Gameplay".
17. Wait 2 seconds.
18. Move the mouse to 540,780.
19. Wait 1 second.
20. Take a screenshot called "mission selected".
21. Click "OK".
22. Wait up to 180 seconds for "UI.log" to say "resources:".
23. Wait 8 seconds.
24. Take a screenshot called "tutorial".
25. Check that the strip of resource cells at the top left is drawn where Fleet Operations' own resource strip sits (x about 75 to 426 of 1600 on the HUD canvas), and that Roots' own counters are not drawn a second time.
26. Drag from 1040,240 to 1190,380.
27. Wait 1 second.
28. Wait 1 second.
29. Wait 3 seconds.
30. Take a screenshot called "selected".
31. Check that the selected ship's command info bar (its name, hull, shields, crew and energy, at the top of the view) is drawn in full, with no cell of the resource strip over it, and that nothing of the strip overlaps Fleet Operations' avatar box or buttons.
32. Expect "UI.log" contains "resources: Dilithium=".
33. Expect "UI.log" contains "Tritanium".
34. Expect "UI.log" contains "Supply".
35. Expect "UI.log" not contain "(Metal)".
36. Expect "UI.log" not contain "(Biomatter)".
37. Expect "UI.log" not contain "PANIC".
38. Expect "Display8.log" not contain "VUID".
39. Expect "Display8.log" not contain "device removed".
40. Quit the game.
41. Expect the game to have exited.
42. Expect no crash.
