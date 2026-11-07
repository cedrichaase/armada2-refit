# Fleet Operations: Roots with the UI: every machined look over the player's HUD, from a save

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh --scene game HUD.Minimap=1 HUD.Info=1 HUD.UnitView=1 HUD.Tooltips=1 HUD.Bar=1 HUD.BarLabels=1 HUD.InfoCamera=1
Timeout: 20 min

Run with `a2test --game fo-roots` and `--install .`. The Instant Action save (Federation against
the Klingon AI, `testbench/bench/roots-save.sh`; `../README.md`, "Roots straight into a map, and
scenes on Roots") puts the game straight on the map with `Theme=auto`, so the first shot is the
fleetops set's look for the Federation (Starfleet); then every machined look is set by name
through the overlay (`UI.UI:Theme`, a live row; `ui/STYLE.md`, "Starfleet" and "The other
factions on Roots"), each over the same HUD. Which race wears which look is the set's
(`ui/content/sets/fleetops/ui.ini`). Scene.asi selects the construction ship so the bar shows.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Move the mouse into the bottom right corner.
4. Move the mouse to 700,300.
5. Wait 1 second.
6. Select the ship of class "Mediterranean" owned by the player.
7. Wait 2 seconds.
8. Take a screenshot called "auto".
9. Expect "UI.log" contains "(Federation)".
10. Overlay UI.UI:Theme=starfleet
11. Wait 2 seconds.
12. Take a screenshot called "starfleet".
13. Overlay UI.UI:Theme=redline
14. Wait 2 seconds.
15. Take a screenshot called "redline".
16. Overlay UI.UI:Theme=jade
17. Wait 2 seconds.
18. Take a screenshot called "jade".
19. Overlay UI.UI:Theme=bronze
20. Wait 2 seconds.
21. Take a screenshot called "bronze".
22. Overlay UI.UI:Theme=circuit
23. Wait 2 seconds.
24. Take a screenshot called "circuit".
25. Overlay UI.UI:Theme=bone
26. Wait 2 seconds.
27. Take a screenshot called "bone".
28. Overlay UI.UI:Theme=circuit-teal
29. Wait 2 seconds.
30. Take a screenshot called "circuit-teal".
31. Overlay UI.UI:Theme=violet
32. Wait 2 seconds.
33. Take a screenshot called "violet".
34. Check that the HUD's panels of ours (the resource strip, the top-right buttons, the bar's grid and the minimap) have near-black glass and dark metal rims, violet appearing only as a thin line along each rim and small lamps (no violet fill), and that the text reads.
35. Expect "UI.log" not contain "PANIC".
36. Expect "Display8.log" not contain "VUID".
37. Expect "Display8.log" not contain "device removed".
38. Quit the game.
39. Expect the game to have exited.
40. Expect no crash.
