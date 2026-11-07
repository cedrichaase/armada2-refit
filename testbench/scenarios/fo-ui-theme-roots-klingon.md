# Fleet Operations: Roots with the UI: the Klingon HUD in its Roots look (`roots-klingon`)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.BarLabels=1 HUD.InfoCamera=1
Timeout: 25 min

Run with `a2test --game fo-roots` and `--install .`. The player plays Klingon against the
Federation AI with `Theme=auto`, so the fleetops set's mapping gives the HUD the Klingon
Roots look (`ui/STYLE.md`, "The other factions on Roots"); the info strip along the foot
is Roots' own, beside ours. The sides are picked as `_fo-sides` picks them, with the
player's race picked in the side list's row for Klingon.

1. Launch the game.
2. Wait for the main menu.
3. Move the mouse to 640,520.
4. Wait 1 second.
5. Click "Instant Action".
6. Wait up to 30 seconds for "LAUNCH".
7. Wait 3 seconds.
8. Move the mouse to shell 270,114.
9. Wait 1 second.
10. Click at shell 278,114.
11. Wait 2 seconds.
12. Move the mouse to shell 278,197.
13. Wait 1 second.
14. Click at shell 278,197.
15. Wait 2 seconds.
16. Move the mouse to shell 270,144.
17. Wait 1 second.
18. Click at shell 278,144.
19. Wait 2 seconds.
20. Move the mouse to shell 278,206.
21. Wait 1 second.
22. Click at shell 278,206.
23. Wait 2 seconds.
24. Take a screenshot called "instant action setup".
25. Expect "Klingon" is visible.
26. Expect "Federation" is visible.
27. Include "_fo-theme-roots".
