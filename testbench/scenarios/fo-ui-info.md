# Fleet Operations: Roots with the UI: the info panel is ours, in Roots' own layout, with Roots' range rings on hover

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan
Overlay: UI.HUD:Info=1 UI.HUD:Tooltips=1 UI.HUD:BottomRow=stacked
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6, "The info panel on Roots" (`PLAN-FLEETOPS.md`,
decisions 440 to 459; `ui/HUD.md`, "The info panel on Fleet Operations: Roots"): with `HUD.Info=1`
FleetOpsHook's info panel keeps running, unseen, and ours draws it in Roots' layout from the
`fleetops` set's `[InfoPanel]`; the pointer stays the panel's, so FleetOpsHook's range rings follow
its own icons. The start of Instant Action as the Federation (`_fo-render3d-skirmish`): the Starbase
at about 780,400, a Saber at about 740,648, two Akiras at about 433,590 and 523,668, the
construction ship at about 803,570. The Saber's four weapon icons sit on its image (the bottom
right one at about 637,902); the sensors icon at about 720,873.

1. Include "_fo-render3d-skirmish".
2. Expect "UI.log" contains "info: ours, in Roots' layout over its panel's unseen Render".
3. Move the mouse to 738,646.
4. Wait 1 second.
5. Click at 740,648.
6. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Single".
7. Move the mouse to 640,150.
8. Wait 2 seconds.
9. Take a screenshot called "one ship".
10. Expect "UI.log" contains "info: Roots' panel 0x".
11. Check that the bottom panel spans from the minimap to the right edge in a metal frame with blue lamps at both ends, and shows the ship's name at the top left, a crew figure with a number, a green bar above a blue bar at the bottom left, a top view of the ship in the middle with four small purple square icons on it, and a block of six blue square icons to its right.
12. Move the mouse to 635,900.
13. Wait 1 second.
14. Move the mouse to 637,902.
15. Wait up to 10 seconds for "UI.log" to say "info: the engine's hover on Roots' weapon icon".
16. Wait 2 seconds.
17. Take a screenshot called "weapon icon hover, the ring".
18. Check that a thin red circle is drawn on the map round the selected ship, and a weapon's tooltip (its name, DPS and range) is shown.
19. Move the mouse to 718,871.
20. Wait 1 second.
21. Move the mouse to 720,873.
22. Wait up to 10 seconds for "UI.log" to say "info: the engine's hover on Roots' system 0 (sensors)".
23. Wait 2 seconds.
24. Take a screenshot called "sensors hover, the ring".
25. Check that a thin green circle is drawn on the map round the selected ship, and a tooltip about the sensors is shown.
26. Move the mouse to 640,150.
27. Wait 1 second.
28. Drag from 380,530 to 570,720.
29. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Group of 2".
30. Move the mouse to 640,150.
31. Wait 2 seconds.
32. Take a screenshot called "two ships".
33. Check that the bottom panel shows two small ship pictures side by side at its left, each with a name under it and two thin bars.
34. Move the mouse to 778,398.
35. Wait 1 second.
36. Click at 780,400.
37. Wait up to 10 seconds for "UI.log" to say "info: Roots' panel ours: Builder".
38. Move the mouse to 640,150.
39. Wait 2 seconds.
40. Take a screenshot called "starbase".
41. Check that the bottom panel shows the station's picture with purple icons on it, and at the right a row of ten numbered slots, the first larger, over a long empty bar.
42. Overlay UI.HUD:Info=0.
43. Wait up to 10 seconds for "UI.log" to say "info: Roots' own".
44. Wait 2 seconds.
45. Take a screenshot called "starbase, Roots' own".
46. Check that the bottom panel is Fleet Operations' own (its own frame art), with the same pictures and slots in the same places.
47. Expect "UI.log" not contain "PANIC".
48. Expect "Display8.log" not contain "VUID".
49. Expect "Display8.log" not contain "device removed".
50. Quit the game.
51. Expect the game to have exited.
52. Expect no crash.
