# Fleet Operations: Roots with the UI: the Federation's Starfleet theme over the player's HUD

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.BarLabels=1 HUD.InfoCamera=1
Overlay: UI.UI:Theme=starfleet-a
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. The Starfleet look (`ui/STYLE.md`,
"Starfleet") on every HUD panel the user plays with (the Refit screen's Minimap, Info,
UnitView, Tooltips, Bar and BarLabels), in a Federation skirmish (`testbench/scenarios/fo-ui-theme-starfleet-wide.md` has 21:9), each variant on
trial in turn through the overlay (`UI.UI:Theme`, a live row). Judged: machined metal with
a fine brushed grain round blue glass, dark blue lights, legible text, the game's icons
reading on the plates, and nothing that moves on its own.

1. Include "_fo-render3d-skirmish".
2. Move the mouse to 700,300.
3. Wait 1 second.
4. Drag from 690,530 to 860,690.
5. Wait 2 seconds.
6. Move the mouse to 700,300.
7. Wait 1 second.
8. Move the mouse to 320,788.
9. Wait 1 second.
10. Move the mouse to 324,790.
11. Wait 2 seconds.
12. Take a screenshot called "hud a".
13. Overlay UI.UI:Theme=starfleet-b
14. Wait 2 seconds.
15. Take a screenshot called "hud b".
16. Overlay UI.UI:Theme=starfleet-c
17. Wait 2 seconds.
18. Take a screenshot called "hud c".
19. Overlay UI.UI:Theme=auto
20. Wait 2 seconds.
21. Take a screenshot called "hud auto".
22. Check that the HUD's panels are machined metal round blue glass, the text reads, and the bar's icons read on their plates.
23. Expect "UI.log" not contain "PANIC".
24. Expect "Display8.log" not contain "VUID".
25. Expect "Display8.log" not contain "device removed".
26. Quit the game.
27. Expect the game to have exited.
28. Expect no crash.
