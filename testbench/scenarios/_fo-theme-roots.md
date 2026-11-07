# Fragment: Fleet Operations: Roots, a skirmish under way with the HUD of the player's race in its theme, shot

Included by the `fo-ui-theme-roots-*` scenarios once their sides are picked (the player's
race against the Federation AI): launch, wait for the first hull, select what stands round
the camera's start, take the HUD, hover the first cell of the bar's grid for its tooltip,
take it again, and quit.

1. Click "LAUNCH".
2. Wait up to 240 seconds for "Render3D.log" to say "(claimed at the instance draw)".
3. Wait 5 seconds.
4. Move the mouse to 700,300.
5. Wait 1 second.
6. Drag from 520,240 to 1010,700.
7. Wait 2 seconds.
8. Move the mouse to 700,300.
9. Wait 1 second.
10. Take a screenshot called "hud".
11. Move the mouse to 46,512.
12. Wait 1 second.
13. Move the mouse to 50,514.
14. Wait 2 seconds.
15. Take a screenshot called "hud hover".
16. Check that the HUD's panels of ours (the resource strip, the top-right buttons, the bar's grid and the minimap) are drawn in the player's race's Roots look, a metal rim round glass in that race's colours, and that the text reads.
17. Expect "UI.log" not contain "PANIC".
18. Expect "Display8.log" not contain "VUID".
19. Expect "Display8.log" not contain "device removed".
20. Quit the game.
21. Expect the game to have exited.
22. Expect no crash.
