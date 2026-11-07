# Fleet Operations: Roots with the UI: the info panel is ours, in Roots' own layout, with Roots' range rings on hover

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Launch: -nointro save\ia-fed-kli.sav
Setup: testbench/bench/roots-save.sh ia-fed-kli display/bench-mode.sh stub Backend=vulkan OneWindow=0 ui/bench-asi.sh --scene game HUD.Info=1
Overlay: UI.HUD:Info=1 UI.HUD:Tooltips=1 UI.HUD:BottomRow=stacked
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The info panel on Roots" (`PLAN-FLEETOPS.md`,
decisions 440 to 459; `ui/HUD.md`, "The info panel on Fleet Operations: Roots"): with `HUD.Info=1`
FleetOpsHook's info panel keeps running, unseen, and ours draws it in Roots' layout from the
`fleetops` set's `[InfoPanel]`; the pointer stays the panel's, so FleetOpsHook's range rings follow
its own icons. The game starts in Instant Action from a save (Federation against the Klingon AI,
`Launch: -nointro save\ia-fed-kli.sav`; `../README.md`, "Roots straight into a map, and scenes on
Roots"), and `Scene.asi` (the `game` scene) selects the units by class through the overview's own
Select (`README.md`, "Objects"). The Saber's four weapon icons sit on its image in the panel (the
bottom right one at about 637,902); the sensors icon at about 720,873. The pointer is pushed into
the corner first, where the bench and the game agree on it (no menu was walked).

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Expect "UI.log" contains "info: ours, in Roots' layout over its panel's unseen Render".
4. Move the mouse into the bottom right corner.
5. Move the mouse to 640,150.
6. Wait 1 second.
7. Select the ship of class "Saber" owned by the player.
8. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Single".
9. Wait 2 seconds.
10. Take a screenshot called "one ship".
11. Scene "camera 2752 6500 14500 2752 0 17628".
12. Expect "UI.log" contains "info: Roots' panel 0x".
13. Check that the bottom panel spans from the minimap to the right edge in a metal frame with blue lamps at both ends, and shows the ship's name at the top left, a crew figure with a number, a green bar above a blue bar at the bottom left, a top view of the ship in the middle with four small purple square icons on it, and a block of six blue square icons to its right.
14. Move the mouse to 635,900.
15. Wait 1 second.
16. Move the mouse to 637,902.
17. Wait up to 10 seconds for "UI.log" to say "info: the engine's hover on Roots' weapon icon".
18. Wait 2 seconds.
19. Take a screenshot called "weapon icon hover, the ring".
20. Check that a thin red circle is drawn on the map round the selected ship, and a weapon's tooltip (its name, DPS and range) is shown.
21. Move the mouse to 718,871.
22. Wait 1 second.
23. Move the mouse to 720,873.
24. Wait up to 10 seconds for "UI.log" to say "info: the engine's hover on Roots' system 0 (sensors)".
25. Wait 2 seconds.
26. Take a screenshot called "sensors hover, the ring".
27. Check that a thin green circle is drawn on the map round the selected ship, and a tooltip about the sensors is shown.
28. Move the mouse to 640,150.
29. Wait 1 second.
30. Scene "camera rts".
31. Select the ships of class "Mediterranean" owned by the player.
32. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Group of 2".
33. Wait 2 seconds.
34. Take a screenshot called "two ships".
35. Check that the bottom panel shows two small ship pictures side by side at its left, each with a name under it and two thin bars.
36. Select the station of class "Starbase" owned by the player.
37. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Builder".
38. Wait 2 seconds.
39. Take a screenshot called "starbase".
40. Check that the bottom panel shows the station's picture with purple icons on it, a grey icon among the blue system icons, and at the right a row of ten numbered slots, the first larger, over a long empty bar.
41. Center the camera on the station of class "Starbase" owned by the player.
42. Move the mouse to 640,520.
43. Wait 1 second.
44. Move the mouse to 643,523.
45. Wait 2 seconds.
46. Take a screenshot called "pointer over the station".
47. Check that a status line about the station under the pointer (its name or class with its shields and hull figures, in Fleet Operations' own small text) is drawn near the top centre of the screen.
48. Overlay UI.HUD:Info=0.
49. Wait up to 10 seconds for "UI.log" to say "info: Roots' own".
50. Move the mouse to 640,150.
51. Wait 2 seconds.
52. Take a screenshot called "starbase, Roots' own".
53. Check that the bottom panel is Fleet Operations' own (its own frame art), with the station's picture, icons and ten slots.
54. Expect "UI.log" not contain "PANIC".
55. Expect "Display8.log" not contain "VUID".
56. Expect "Display8.log" not contain "device removed".
57. Quit the game.
58. Expect the game to have exited.
59. Expect no crash.
