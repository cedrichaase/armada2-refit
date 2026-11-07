# Fleet Operations: Roots: the status line about the craft under the pointer drawn by Roots (the A side of fo-ui-hover)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Hover=0 HUD.LayoutLog=1
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .` (PLAN-FLEETOPS.md, decisions 460-469). Roots'
defaults with the strip left to the engine (`HUD.Hover=0`); `LayoutLog=1` puts the hook in only to
log what the engine draws (`hover: craft ... (drawn by the engine)`), the measurement for ours. The
same pointer moves as `fo-ui-hover`: nothing, the starbase (at once and after two seconds), off
again, eight places where the start fleet's ships stand on the maps seen, and the starbase with
the fleet selected.

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Move the mouse to 700,300.
4. Wait 1 second.
5. Take a screenshot called "nothing under the pointer".
6. Move the mouse to 838,378.
7. Wait 1 second.
8. Move the mouse to 840,380.
9. Wait 0.3 seconds.
10. Take a screenshot called "starbase at once".
11. Wait 2 seconds.
12. Take a screenshot called "starbase".
13. Move the mouse to 700,300.
14. Wait 1 second.
15. Take a screenshot called "pointer moved off".
16. Move the mouse to 418,598.
17. Wait 1 second.
18. Move the mouse to 420,600.
19. Wait 2 seconds.
20. Take a screenshot called "ship 1".
21. Move the mouse to 810,573.
22. Wait 1 second.
23. Move the mouse to 812,575.
24. Wait 2 seconds.
25. Take a screenshot called "ship 2".
26. Move the mouse to 513,678.
27. Wait 1 second.
28. Move the mouse to 515,680.
29. Wait 2 seconds.
30. Take a screenshot called "ship 3".
31. Move the mouse to 746,653.
32. Wait 1 second.
33. Move the mouse to 748,655.
34. Wait 2 seconds.
35. Take a screenshot called "ship 4".
36. Move the mouse to 333,638.
37. Wait 1 second.
38. Move the mouse to 335,640.
39. Wait 2 seconds.
40. Take a screenshot called "ship 5".
41. Move the mouse to 878,608.
42. Wait 1 second.
43. Move the mouse to 880,610.
44. Wait 2 seconds.
45. Take a screenshot called "ship 6".
46. Move the mouse to 788,723.
47. Wait 1 second.
48. Move the mouse to 790,725.
49. Wait 2 seconds.
50. Take a screenshot called "ship 7".
51. Move the mouse to 463,738.
52. Wait 1 second.
53. Move the mouse to 465,740.
54. Wait 2 seconds.
55. Take a screenshot called "ship 8".
56. Move the mouse to 700,300.
57. Wait 1 second.
58. Drag from 30,120 to 1250,640.
59. Wait 3 seconds.
60. Move the mouse to 838,378.
61. Wait 1 second.
62. Move the mouse to 840,380.
63. Wait 2 seconds.
64. Take a screenshot called "starbase, fleet selected".
65. Expect "UI.log" contains "(drawn by the engine)".
66. Expect "UI.log" not contain "PANIC".
67. Expect "Display8.log" not contain "VUID".
68. Expect "Display8.log" not contain "device removed".
69. Quit the game.
70. Expect the game to have exited.
71. Expect no crash.
