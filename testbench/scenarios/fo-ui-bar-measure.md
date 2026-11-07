# Fleet Operations: Roots with the UI: the button bar measured (draws nothing), as FleetOpsHook lays it out

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Enable=0
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. FO-6 step 5 (`PLAN-FLEETOPS.md`,
"FO-6: the UI on Roots", "Step 5: the bar measured") draws nothing of the bar: with the
HUD part off, as on Roots by default, the UI only logs what the engine's own objects say,
once per change and capped (`ui/src/engine/palette.rs`, `log_measure`): `bar: measure`
(the mode, its columns and rows, how many of the 64 buttons are shown), `bar:  slots`
(each shown button: its slot, kind, name, state) and `bar: hover` (the button under the
pointer, its states and who owns the pointer). The scenario selects the player's two
construction ships and the station with a box, hovers the bar's buttons and reads what the
log says; whether the Tractor Beam ring is drawn is judged (`fo-render3d-rings` does the
same with hulls claimed).

1. Include "_fo-render3d-skirmish".
2. Move the mouse to 700,300.
3. Wait 1 second.
4. Drag from 30,120 to 1250,640.
5. Wait up to 20 seconds for "UI.log" to say "Weapon:all_bastion".
6. Expect "UI.log" contains "bar: measure mode=1".
7. Expect "UI.log" contains "use=14x20 floating=3x7".
8. Expect "UI.log" contains "of 64 (palette".
9. Take a screenshot called "box selected".
10. Move the mouse to 884,788.
11. Wait 1 second.
12. Move the mouse to 880,790.
13. Wait up to 10 seconds for "UI.log" to say "bar: hover slot".
14. Wait 2 seconds.
15. Take a screenshot called "bastion hover, no ring".
16. Move the mouse to 700,300.
17. Wait 1 second.
18. Drag from 290,560 to 520,745.
19. Wait up to 20 seconds for "UI.log" to say "Menu:form:s1 6:Menu:aiMenu:s1 7:Weapon:npc_tractorbeam".
20. Move the mouse to 810,788.
21. Wait 1 second.
22. Move the mouse to 812,790.
23. Wait up to 10 seconds for "UI.log" to say "hover slot 7 Weapon npc_tractorbeam".
24. Wait 2 seconds.
25. Take a screenshot called "tractor ring, ships only".
26. Check that a thin red circle is drawn on the map round the selected ships, its radius about a quarter of the screen's width, with the Tractor Beam tooltip beside the pointer.
27. Move the mouse to 320,788.
28. Wait 1 second.
29. Move the mouse to 322,790.
30. Wait up to 10 seconds for "UI.log" to say "hover slot 0 Command repair".
31. Wait 2 seconds.
32. Take a screenshot called "repair, no ring".
33. Check that no red circle is drawn on the map.
34. Move the mouse to 600,788.
35. Wait 1 second.
36. Click at 602,790.
37. Wait up to 20 seconds for "UI.log" to say "Build:fed_outpost".
38. Expect "UI.log" contains "Command:cancel".
39. Take a screenshot called "construction ship build list".
40. Move the mouse to 700,300.
41. Wait 1 second.
42. Drag from 680,260 to 980,480.
43. Wait up to 20 seconds for "UI.log" to say "Command:recrew@0,1:s0 1:Menu:order".
44. Move the mouse to 529,788.
45. Wait 1 second.
46. Click at 531,790.
47. Wait up to 20 seconds for "UI.log" to say "Build:fed_saber".
48. Take a screenshot called "station build list".
49. Expect "UI.log" contains "Build:fed_mediterranean".
50. Expect "UI.log" not contain "standing down: the bar".
51. Expect "UI.log" not contain "PANIC".
52. Expect "Display8.log" not contain "VUID".
53. Expect "Display8.log" not contain "device removed".
54. Quit the game.
55. Expect the game to have exited.
56. Expect no crash.
