# An Armada2.exe no profile knows: every plugin stands down, the game runs as stock

Resolutions: 1280x720
Mod: refit
Setup: tools/engine/bench-exe.sh stamp display/bench-mode.sh stub
Timeout: 25 min

Run with `--install .`. `tools/engine/bench-exe.sh stamp` changes one bit of the PE
timestamp of the clone's `Armada2.exe` (the clone's own copy, moved over its name; the
install is never written), so the executable's identity is no build's. Every plugin on
the symbol profile (`crates/a2engine/README.md`) logs `engine: unknown build`, stands
every hook down with that reason and leaves the game as stock: Display8 takes the
bail-out (display 1.0.0: the system's own Direct3D 8; it forwarded to the old chain
before), so the plugins are told no backend, UI.asi draws nothing, and the settings
screen shows the plugin note.
The shell is the stock one (4:3, centred in the window with black sides; the Instant
Action cube is clicked in screen pixels), then a skirmish loads and ticks.

The plugins on a profile at this point are Display8, UI.asi and Render3D.asi. Each later package of
phase 8 (the C plugins, render3d) appends its plugin's section here: `engine: unknown
build`, its note, no `installed` hook line, no `read before its hook`.

## Main menu
1. Launch the game.
2. Wait up to 300 seconds for the main menu.
3. Wait 5 seconds.
4. Take a screenshot called "main menu".
5. Expect "Display8.log" contains "engine: Armada2.exe timestamp 0x3c4c76bc, image size 0x3fa000".
6. Expect "Display8.log" contains "engine: unknown build".
7. Expect "Display8.log" contains "engine: display.modes stands down: ".
8. Expect "Display8.log" contains " unknown build".
9. Expect "Display8.log" not contains "engine: build armada2/gog-1.1".
10. Expect "Display8.log" not contains "read before its hook".
11. Expect "UI.log" contains "engine: unknown build".
12. Expect "UI.log" contains " stands down: ".
13. Expect "UI.log" not contains "engine: build armada2/gog-1.1".
14. Expect "UI.log" not contains "read before its hook".
15. Expect "UI.log" not contains "PANIC".
16. Expect "Display8.log" not contains "PANIC".
## Mission
17. Click at 930,100.
18. Wait for "GAME SETUP".
19. Wait 2 seconds.
20. Click "LAUNCH".
21. Wait 60 seconds.
22. Take a screenshot called "mission".
23. Check that the screenshot shows a space battle map of Star Trek: Armada II drawn by the game (stars, ships or stations, and the game's own interface), not a black or garbled screen and not a second copy of the interface drawn over the game's.
24. Expect "Display8.log" contains "bail-out: engine: unknown build; the game draws through ".
25. Expect "Display8.log" contains "the plugins stand down".
26. Expect "Display8.log" not contains "stub: the backend is".
27. Expect "UI.log" not contains "hud: panels of ours".
28. Expect "UI.log" not contains "installed".
29. Expect "Display8.log" not contains "installed".
30. Expect "Render3D.log" contains "engine: unknown build".
31. Expect "Render3D.log" contains "stands down".
32. Expect "Render3D.log" not contains "engine: build armada2/gog-1.1".
33. Expect "Render3D.log" not contains "installed".
34. Expect "Render3D.log" not contains "read before its hook".
35. Expect "Render3D.log" not contains "PANIC".
36. Expect the game is still running.
37. Quit the game.
38. Expect no crash.
