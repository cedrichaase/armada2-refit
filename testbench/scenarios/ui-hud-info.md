# The info panel is the UI's own (UI.asi, [HUD] Info=1)

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: ui/bench-hud-info.sh UI.Theme=borg
Timeout: 12 min

`ui/HUD.md`, "The info panel". With `[HUD] Info=1` the UI draws the info panel itself
at the bottom centre: one object (wireframe, name, class, race emblem, hull and shield
bars, crew, officers and special energy, its five systems), a builder with its queue
of ten, or a group of icons; the stock panel keeps running but neither draws nor takes
the mouse. The UI's bar is on too, and placed against the panel. The scene
(`testbench/scene/scenes/ui-hud-info.ini`) has the player's Galaxy (`mine`), three
destroyers (`d1`..`d3`, the Klingons `foe` and `foe2` pick on `d1`, which cannot die),
a starbase (`base`), a dilithium moon (`moon`) and the Klingons. `Theme=borg` fixes
the look, so the clicks below land on the same places whatever race the bench's HUD
gets (the race is the mission's, and `UI.log` names it: "race: the HUD's race is").

At 1280x720 in the Borg theme: the starbase's queue is the bottom row of the panel,
slot 3 at 579,667; the group's three icons sit at 597, 640 and 683 on row 688 (the
panel moved right for the bar beside the minimap, and its group tiles are small:
HUD.md, "The bottom row"). `UI.log`
names each object the panel describes once ("info: Single <name> | <class> | ..."),
each tooltip ("info: tooltip ..."), and each press ("info: cancel queue slot 2",
"info: select ...").

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "UI.log" contains "info: ours".
5. Expect "UI.log" contains "race: the HUD's race is".
6. Scene "select mine".
7. Wait 1 second.
8. Move the mouse to 1000,150.
9. Expect "UI.log" contains "Galaxy Class".
10. Take a screenshot called "one unit".
11. Check that at the bottom centre there is one dark square-cornered panel with green corner brackets, holding at its left a square picture of a starship drawn in 3D against space (the close-up, `[HUD] InfoCamera`), a ship's name in capitals and "Galaxy Class" below it, two thin bars each with an icon at the left and "100%" at the right, a row of small green numbers with icons, and a row of five boxes along the bottom each with an icon and a percentage; and that a grid of command icons sits above or beside it.
12. Scene "select moon".
13. Wait 1 second.
14. Expect "UI.log" contains "Dilithium Moon".
15. Scene "select foe".
16. Wait 1 second.
17. Expect "UI.log" contains "Negh'Var Class".
18. Take a screenshot called "enemy".
19. Check that the panel at the bottom now names a Klingon ship of the "Negh'Var Class" and shows it in 3D at the left.
20. Scene "select base".
21. Wait 1 second.
22. Expect "UI.log" contains "info: Builder".
23. Press q.
24. Wait 1 second.
25. Press q.
26. Press q.
27. Press q.
28. Press q.
29. Wait 1 second.
30. Move the mouse to 579,667.
31. Wait 2 seconds.
32. Take a screenshot called "queue".
33. Check that the panel's bottom row shows several small ship wireframes in boxes, the first outlined, with a thin progress bar under it, and a tooltip naming the ship is shown above the panel.
34. Click at 579,667.
35. Wait 1 second.
36. Expect "UI.log" contains "info: cancel queue slot 2".
37. Wait 60 seconds.
38. Scene "select d1 d2 d3".
39. Wait 1 second.
40. Move the mouse to 640,688.
41. Wait 2 seconds.
42. Take a screenshot called "group".
43. Check that the panel at the bottom shows three boxes side by side, each with a small ship wireframe and a thin bar under it, and a tooltip naming one destroyer and its class above them.
44. Click at 597,688.
45. Wait 1 second.
46. Expect "UI.log" contains "info: select".
47. Expect scene "selection" answers "selected 1".
48. Move the mouse to 1000,150.
49. Wait 1 second.
50. Take a screenshot called "damaged".
51. Check that the panel shows one destroyer in 3D at the left, with its hull bar short and red or orange, and some of its five system boxes showing red percentages below 100%.
52. Move the mouse to 770,270.
53. Wait 1 second.
54. Take a screenshot called "hover".
55. Check that a small card naming "Starbase" with a bar is shown beside the panel at the bottom, at its right, or above the panel's right end.
56. Expect the game is still running.
57. Expect no crash.
