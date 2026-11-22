# Fleet Operations: Roots with the UI's forms but no specs: Save, then Load from a mission, answered Yes

Resolutions: 1600x900
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh Menus.Forms=1 Menus.Specs=0 Menus.Scene=1 Menus.Enable=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. A reference for
`fo-ui-dialogs-saves` (PLAN-FLEETOPS.md, decision 255): the forms ours but no spec
(`[Menus] Specs=0`: the adopted dialogs are their pictures, centred, the forms generic), a skirmish saved from the in-mission menu, then Load Savegame, the
save picked, Load, and Yes to Roots' question whether to end the mission. With the pictures
the save loads after Yes (`20261107-054525`), which is what `fo-ui-dialogs-saves` checks of the
specs (decision 255). The clicks on the dialogs are at their places in the design space,
centred at 1600x900 (200 pixels in), each after the pointer rested on it.

1. Launch the game.
2. Wait for "Instant Action".
3. Wait 3 seconds.
4. Click "Instant Action".
5. Wait up to 30 seconds for "LAUNCH".
6. Wait 3 seconds.
7. Move the mouse to 790,872.
8. Wait 1 second.
9. Move the mouse to 800,875.
10. Wait 2 seconds.
11. Click at 800,875.
12. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
13. Wait 10 seconds.
14. Move the mouse to 900,300.
15. Press Escape.
16. Wait up to 20 seconds for "Save Game".
17. Wait 2 seconds.
18. Take a screenshot called "options".
19. Click "Save Game".
20. Wait 5 seconds.
21. Take a screenshot called "save".
22. Move the mouse to 800,860.
23. Wait 1 second.
24. Move the mouse to 810,863.
25. Wait 1 second.
26. Click at 810,863.
27. Wait 5 seconds.
28. Take a screenshot called "saved".
29. Move the mouse to 770,500.
30. Wait 1 second.
31. Move the mouse to 778,503.
32. Wait 1 second.
33. Click at 778,503.
34. Wait 3 seconds.
35. Move the mouse to 290,860.
36. Wait 1 second.
37. Move the mouse to 298,863.
38. Wait 1 second.
39. Click at 298,863.
40. Wait 3 seconds.
41. Click "Load Savegame".
42. Wait 5 seconds.
43. Take a screenshot called "load".
44. Move the mouse to 790,110.
45. Wait 1 second.
46. Move the mouse to 798,114.
47. Wait 1 second.
48. Click at 798,114.
49. Wait 1 second.
50. Move the mouse to 800,860.
51. Wait 1 second.
52. Move the mouse to 810,863.
53. Wait 1 second.
54. Click at 810,863.
55. Wait 4 seconds.
56. Take a screenshot called "load asks".
57. Click "Yes".
58. Wait up to 60 seconds for "UI.log" to say "s1.sav".
59. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
60. Wait 5 seconds.
61. Take a screenshot called "loaded".
62. Check that the skirmish is under way again: a 3D space view with the game's HUD, no menu or loading screen.
63. Stop the game.
64. Expect no crash.
