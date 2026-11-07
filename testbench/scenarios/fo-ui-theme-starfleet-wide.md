# Fleet Operations: Roots with the UI: the Starfleet theme at 21:9, for the metal's detail

Resolutions: 2560x1080
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.BarLabels=1 HUD.InfoCamera=1
Overlay: UI.UI:Theme=starfleet-a
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. The Starfleet look (`ui/STYLE.md`,
"Starfleet") on every HUD panel the user plays with (the Refit screen's Minimap, Info,
UnitView, Tooltips, Bar and BarLabels), in a skirmish at 21:9 (the race Instant Action deals: the sides list does not open at this
size on the bench, and the theme is set by name, so it shows whatever the race), each variant on
trial in turn through the overlay (`UI.UI:Theme`, a live row). Judged: machined metal with
a fine brushed grain round blue glass, dark blue lights, legible text, the game's icons
reading on the plates, and nothing that moves on its own.

1. Launch the game.
2. Wait for the main menu.
3. Move the mouse to 1280,494.
4. Wait 1 second.
5. Click at 1280,494.
6. Wait up to 30 seconds for "LAUNCH".
7. Wait 3 seconds.
8. Click "LAUNCH".
9. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
10. Wait 5 seconds.
11. Move the mouse to 1280,300.
12. Wait 1 second.
13. Drag from 590,470 to 2500,720.
14. Wait 3 seconds.
15. Take a screenshot called "wide a".
16. Overlay UI.UI:Theme=starfleet-b
17. Wait 2 seconds.
18. Take a screenshot called "wide b".
19. Overlay UI.UI:Theme=starfleet-c
20. Wait 2 seconds.
21. Take a screenshot called "wide c".
22. Expect "UI.log" not contain "PANIC".
23. Expect "Display8.log" not contain "VUID".
24. Expect "Display8.log" not contain "device removed".
25. Quit the game.
26. Expect the game to have exited.
27. Expect no crash.
