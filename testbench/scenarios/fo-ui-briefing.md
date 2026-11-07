# Fleet Operations: Roots with the UI: the tutorial's briefing at Roots' own layout, with no panel of ours over it

Resolutions: 2150x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.BarLabels=1 HUD.InfoCamera=1 HUD.LayoutLog=1
Timeout: 14 min

The user's report of 2026-11-07 (Roots 5.0Z11, 3440x1440, the panels of the player's
`RefitPlayer.ini` on beyond Roots' defaults: minimap, info panel, unit view, tooltips, bar
and its labels): in the first allied tutorial the briefing and objectives screen was
"very stretched", and every panel of ours was drawn over it. The screen is Roots' own
(the engine's `ObjectivesDisplay`, `[HUD] Messages=0` on Roots), laid out on the canvas of
Roots' 16:9 interface files; `fo-ui-briefing-ref` shows it the same width with no
`UI.asi` at all (PLAN-FLEETOPS.md, decisions 320 onward). While the engine's box shows,
the panels of ours stand aside (`hud: under the engine's box`) and come back when OK
closes it. Compare the "briefing" shot with `fo-ui-briefing-ref`'s.

The way in is `fo-render3d-tutorial`'s, with Single Player clicked at its place five
seconds after the main menu shows: clicked by its label sooner, it was missed in two runs
of three on this branch.

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
24. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
25. Wait 8 seconds.
26. Take a screenshot called "tutorial".
27. Press Alt+o.
28. Wait 3 seconds.
29. Take a screenshot called "briefing".
30. Check that the briefing screen (BRIEFING SUMMARY, OBJECTIVES, an OK button) is drawn in Roots' metal frame, as wide as in fo-ui-briefing-ref's "briefing" shot (from about x 65 to 2000 of 2150), and that none of the panels of ours (the resource strip, the round buttons at the top right, the minimap, the button bar, the info panel, the unit view) is drawn over it.
31. Expect "UI.log" contains "hud: under the engine's box".
32. Click "OK".
33. Wait 3 seconds.
34. Take a screenshot called "closed".
35. Check that the briefing screen is gone and the panels of ours (the resource strip, the minimap, the button bar) are drawn again.
36. Expect "UI.log" contains "the engine's box closed".
37. Expect "UI.log" not contain "PANIC".
38. Expect "Display8.log" not contain "VUID".
39. Expect "Display8.log" not contain "device removed".
40. Quit the game.
41. Expect the game to have exited.
42. Expect no crash.
