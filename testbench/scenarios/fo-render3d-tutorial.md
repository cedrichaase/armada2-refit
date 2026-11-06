# Fleet Operations: Roots with Render3D: the Enterprise-E of the first allied tutorial is drawn

Resolutions: 1280x960
Mod: refit
Layers: display asiloader render3d
Setup: display/bench-mode.sh stub Backend=vulkan
Timeout: 12 min

The user's report of 2026-11-06: in Roots' first allied tutorial (*TUTORIAL: Gameplay*)
the player's Enterprise-E (`fenterprise`, `fed_sovereignent.sod`) was not drawn at all,
with render3d's hulls claiming it at the instance draw. The claim honours what the engine
wrote into the class's shared database for the instance (`hulls::claim::honour_database`),
and the database marked the `lod*` level the module had picked as hidden: those flags are
the engine's own choice of level, which can differ from the module's, so the whole hull
went with the level (`hulls: fed_sovereignent: 4 of 4 shown nodes`: root, `damage`,
`geometry` and `lod1`, none of the meshes). The module now keeps its own pick for a
level of detail and logs the database's flag (`... a level of detail, the module's own
pick kept`); every other node's flags are honoured as before.

The mission is reached through the single-player screen: the allied (alpha) tile, then
the mission list. Roots' shell under Display8 takes a click only after the pointer has
been over the spot, so the item and OK are clicked by their labels after the pointer has been moved
over them. The list item is double-clicked: a single click only highlights it under the
pointer, and OK with nothing chosen closes the list. The wait is for
render3d's claim of the Enterprise, then the judged shot: the ship stands near the top
centre of the opening view, above the wormhole's blue glow.

1. Launch the game.
2. Wait for the main menu.
3. Move the mouse to 640,393.
4. Wait 1 second.
5. Click "Single Player".
6. Wait 4 seconds.
7. Move the mouse to 235,225.
8. Wait 1 second.
9. Click at 237,225.
10. Wait up to 20 seconds for "TUTORIAL: Gameplay".
11. Wait 2 seconds.
12. Move the mouse to 640,233.
13. Wait 1 second.
14. Click "TUTORIAL: Gameplay".
15. Wait 1 second.
16. Double-click "TUTORIAL: Gameplay".
17. Wait 2 seconds.
18. Move the mouse to 540,780.
19. Wait 1 second.
20. Take a screenshot called "mission selected".
21. Click "OK".
22. Wait up to 180 seconds for "Render3D.log" to say "fed_sovereignent at".
23. Wait 8 seconds.
24. Take a screenshot called "tutorial".
25. Check that a starship hull (a grey Federation ship with a saucer and two nacelles) is drawn in the upper middle of the view, above the blue glow, and not just a small yellow marker on empty space.
26. Expect "Render3D.log" contains "hulls: fenterprise is of race 'unitedfederation'".
27. Expect "Render3D.log" not contain "hulls: fed_sovereignent: 4 of 4 shown nodes".
28. Expect "Display8.log" not contain "VUID".
29. Expect "Display8.log" not contain "device removed".
30. Quit the game.
31. Expect the game to have exited.
32. Expect no crash.
