# Backend=none: the bail-out on purpose, the stock game through the system's Direct3D 8

Resolutions: 1280x720
Mod: refit
Setup: display/bench-mode.sh stub Backend=none
Timeout: 20 min

Run with `--install .`. `Backend=none` (display 1.0.0) takes the bail-out at
`Direct3DCreate8` (`display/README.md`, "The bail-out"): Display8 loads the system's own
`d3d8.dll` (`GetSystemDirectoryA`; under Proton, Wine's), hands the engine its
`IDirect3D8` itself, and tells the plugins no backend, so render3d and the UI stand
down and the game is the stock one. It is what a player gets when neither Vulkan nor
Direct3D 12 can start, and the way to see it outside the bench: set
`[Display8.Display8] Backend=none` in `RefitPlayer.ini`. With the UI standing down the shell
is the stock one, 800x600 at the screen's top left, so the Instant Action cube is
clicked in screen pixels.

    ./a2test run display-bail-out --install . -j 1

## Start
1. Launch the game with "".
2. Wait up to 120 seconds for "Display8.log" to say "Direct3DCreate8(".
3. Expect "Display8.log" contains "Backend=none: the bail-out (the system's Direct3D 8)".
4. Expect "Display8.log" contains "bail-out: Backend=none; the game draws through ".
5. Expect "Display8.log" contains "d3d8.dll; the plugins stand down".
6. Expect "Display8.log" contains "-> the system's ".
7. Expect "Display8.log" not contain "stub: the backend is".
8. Expect "Display8.log" not contain "probe: ".
9. Wait 8 seconds.
10. Take a screenshot called "intro".
11. Press Escape.
12. Wait 3 seconds.
13. Press Escape.
14. Wait 3 seconds.
15. Press Escape.
16. Wait 3 seconds.
17. Press Escape.
## Main menu
18. Wait up to 300 seconds for the main menu.
19. Wait 3 seconds.
20. Take a screenshot called "main menu".
21. Check that the screenshot shows the main menu of Star Trek: Armada II (Single Player, Instant Action, Multiplayer, Options ...), not a black or garbled screen.
## Mission
22. Click at 640,90.
23. Wait for "GAME SETUP".
24. Wait 2 seconds.
25. Click "LAUNCH".
26. Wait 60 seconds.
27. Take a screenshot called "match".
28. Check that the screenshot shows a space battle map of Star Trek: Armada II drawn by the game (stars, ships or stations, and the game's own interface), not a black or garbled screen and not a second copy of the interface drawn over the game's.
29. Expect "Render3D.log" not contain "frame: targets".
30. Expect "UI.log" contains "runs stock: no ".
31. Expect "Display8.log" not contain "PANIC".
32. Expect the game is still running.
33. Quit the game.
34. Expect no crash.
