# Fleet Operations: Roots with the UI: the Federation's Starfleet theme over the player's HUD

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.BarLabels=1 HUD.InfoCamera=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. The Starfleet look (`ui/STYLE.md`,
"Starfleet") on every HUD panel the user plays with (the Refit screen's Minimap, Info,
UnitView, Tooltips, Bar and BarLabels), in a Federation skirmish (`testbench/scenarios/fo-ui-theme-starfleet-wide.md` has 21:9), with `Theme=auto`, so the
fleetops set's mapping gives the Federation the look. Judged: dark brushed metal round
smoked glass with no scan lines, blue lamps only on the info panel (as on Roots), legible text, the game's icons
reading on the plates, and nothing that moves on its own.

1. Include "_fo-render3d-skirmish".
2. Move the mouse to 700,300.
3. Wait 1 second.
4. Drag from 520,240 to 1010,700.
5. Wait 2 seconds.
6. Move the mouse to 700,300.
7. Wait 1 second.
8. Take a screenshot called "hud".
9. Wait 1 second.
10. Move the mouse to 46,512.
11. Wait 1 second.
12. Move the mouse to 50,514.
13. Wait 2 seconds.
14. Take a screenshot called "hud hover".
15. Wait 2 seconds.
16. Check that the HUD's panels of ours are grey metal rims round dark smoked glass with no scan lines, that the resource strip has small blue lamps in its side rims lighting its glass blue from the ends while the top-right buttons, the command grid and the minimap have no lamps, that the text reads, and that the bar's icons read on their plates.
17. Expect "UI.log" not contain "PANIC".
18. Expect "Display8.log" not contain "VUID".
19. Expect "Display8.log" not contain "device removed".
20. Quit the game.
21. Expect the game to have exited.
22. Expect no crash.
