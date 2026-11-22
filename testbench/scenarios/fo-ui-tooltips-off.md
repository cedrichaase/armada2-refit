# Fleet Operations: Roots with the UI on and the tooltips at the pointer Roots' own (the A side of fo-ui-tooltips)

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d ui
Setup: display/bench-mode.sh stub Backend=vulkan ui/bench-asi.sh HUD.Tooltips=0
Timeout: 14 min

Run with `a2test --game fo-roots` and `--install .`. Roots' defaults with the tooltips at the
pointer left to the engine (`HUD.Tooltips=0`, as Roots ships), for the judge to set beside
`fo-ui-tooltips`'s shots: the same box selection of the start fleet, the same hovers (the
Tractor Beam button, the starbase, a station button with a cost line), Roots' own drawing of
its text, the icons and red figures of its font included.

1. Include "_fo-render3d-skirmish".
2. Wait 5 seconds.
3. Move the mouse to 700,300.
4. Wait 1 second.
5. Drag from 30,120 to 1250,640.
6. Wait 3 seconds.
7. Take a screenshot called "selected".
8. Move the mouse to 1162,788.
9. Wait 1 second.
10. Move the mouse to 1164,790.
11. Wait 4 seconds.
12. Take a screenshot called "tractor beam tooltip".
13. Expect "400" is visible.
14. Move the mouse to 700,300.
15. Wait 1 second.
16. Move the mouse to 840,380.
17. Wait 4 seconds.
18. Take a screenshot called "unit tooltip".
19. Move the mouse to 700,300.
20. Wait 1 second.
21. Move the mouse to 882,788.
22. Wait 1 second.
23. Move the mouse to 884,790.
24. Wait 4 seconds.
25. Take a screenshot called "station button tooltip".
26. Expect "UI.log" not contain "PANIC".
27. Expect "Display8.log" not contain "VUID".
28. Expect "Display8.log" not contain "device removed".
29. Quit the game.
30. Expect the game to have exited.
31. Expect no crash.
