# The Orbital Processing Facility over its planet: green, its cells not crossed (#156)

Resolutions: 1280x720
Mod: refit
Setup: testbench/scene/bench-setup.sh game Fog=0
Timeout: 12 min

`ui/HUD.md`, "The placement markers"; issue #156. The Orbital Processing Facility (the
station that mines metal from a planet, `classLabel = "p_mining"`) is placed over a
planet: its class asks for a planet within reach that can take one, never the placement
grid, so the engine finds the place clear over the planet's own taken cells. The UI
drew those cells red and crossed under the green footprint, which read as "every cell
taken". Now a taken cell is crossed only under a blocked footprint. Run it with
`--install .`.

An Instant Action match on Warzone as the Romulans against an easy AI (Game Setup is the
UI's: `_enter-instant-action` has the details). A Romulan construction ship of the
player's (`Scene "spawn ... player"`; a match deals the player's team by race and slot)
is spawned beside the class M planet at 2751,157,2182 and the camera centred on it at
zoom 0.7, so the planet is at the top right. The bar's keys open its build list (Q) and
pick the facility (F). With the pointer over the planet (830,150) the engine finds the
place clear and the ghost sits at the planet's edge; beside it (640,250) it is blocked.
`UI.log` says each time the footprint changes ("placement: footprint clear, 0 crossed,
318 cells not clear, 25 under the footprint, drawn in the theme"): over the planet the
footprint is clear and crosses none of the planet's cells under it (before the fix it
crossed all 25), and the click starts the facility (`Render3D.log`, "construction:
rorbital at"). The two shots are for a person to look at: red and hatched beside the
planet, green with corner brackets over amber tiles at its edge.

1. Launch the game.
2. Wait for the main menu.
3. Click the instant action cube.
4. Wait for "GAME SETUP".
5. Click "Computer AI:Medium".
6. Wait 1 second.
7. Click "Computer AI:Easy".
8. Wait 1 second.
9. Click at hud 767,203.
10. Wait 1 second.
11. Click "Romulan".
12. Wait 1 second.
13. Expect "Romulan" is visible.
14. Click "LAUNCH".
15. Wait up to 90 seconds for "Scene.log" to say "scene ready".
16. Expect "UI.log" not contain "RenderFootprints' draws are not stock".
17. Note "a Romulan construction ship of the player's beside the class M planet at 2751,157,2182"
18. Scene "spawn con rconst 2400 0 1800 0 player".
19. Wait 2 seconds.
20. Scene "select con; center con; zoom 0.7".
21. Wait 2 seconds.
22. Move the mouse to 640,450.
23. Press q.
24. Wait 1 second.
25. Press f.
26. Wait 1 second.
27. Move the mouse to 640,250.
28. Wait up to 20 seconds for "UI.log" to say "placement: footprint blocked".
29. Wait 2 seconds.
30. Take a screenshot called "beside-planet".
31. Move the mouse to 830,150.
32. Wait up to 20 seconds for "UI.log" to say "placement: footprint clear".
33. Expect "UI.log" contains "placement: footprint clear, 0 crossed".
34. Expect "UI.log" not contains "placement: footprint clear, 0 crossed, 0 cells not clear".
35. Wait 2 seconds.
36. Take a screenshot called "over-planet".
37. Click at 830,150.
38. Wait up to 120 seconds for "Render3D.log" to say "construction: rorbital at".
39. Expect the game is still running.
40. Quit the game.
41. Expect no crash.
