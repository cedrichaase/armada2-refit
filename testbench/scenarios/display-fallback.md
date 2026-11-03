# The stub falls back: Vulkan and Direct3D 12 fail at the first CreateDevice, the bail-out draws

Resolutions: 1280x720
Reference: 1280x720
Mod: refit
Setup: display/bench-mode.sh stub Backend=auto FailBackend=vulkan,d3d12
Timeout: 20 min

Run with `--install .`: Display8 with `Backend=auto`: Vulkan, then Direct3D 12, then the
bail-out (`display/README.md`, "Choosing a backend", "The bail-out").
`FailBackend=vulkan,d3d12`, honoured only in a bench clone, makes Vulkan and Direct3D
12 fail on purpose once each one's device, swap chain and frame are all made, so both
pass the probes at `Direct3DCreate8` and the engine's first `CreateDevice` has to take
each down whole. With none left it takes the late bail-out: the stub's `IDirect3D8`
becomes a forward of the system's own, the engine gets the system's device on the same
window, without noticing, and the plugins are told no backend and stand down. Direct3D
12 runs on the GPU here (vkd3d-proton). The game is then the stock one: the shell
800x600 at the screen's top left (the Instant Action cube clicked in screen pixels),
then a skirmish.

    ./a2test run display-fallback --install . -j 1

Other landings, with the backend-specific lines below changed to match, through
`display/stub-scenario.sh display-fallback stub FailBackend=vulkan` (Vulkan fails,
Direct3D 12 draws).

1. Launch the game with "".
2. Wait up to 120 seconds for "Display8.log" to say "bail-out: the system's CreateDevice".
3. Expect "Display8.log" contains "probe: Vulkan can start".
4. Expect "Display8.log" contains "probe: Direct3D 12 can start".
5. Expect "Display8.log" contains "Vulkan could not start: FailBackend=vulkan".
6. Expect "Display8.log" contains "vulkan: what was made is destroyed again".
7. Expect "Display8.log" contains "Direct3D 12 could not start: FailBackend=d3d12".
8. Expect "Display8.log" contains "d3d12: what was made is released again".
9. Expect "Display8.log" contains "stub: no backend could start (vulkan, then d3d12)".
10. Expect "Display8.log" contains "bail-out: no backend could start at the engine's first CreateDevice; the game draws through ".
11. Expect "Display8.log" contains "bail-out: the system's CreateDevice -> 0x0".
12. Expect "Display8.log" not contain "after vulkan, then d3d12 could not start".
13. Wait 8 seconds.
14. Take a screenshot called "intro".
15. Press Escape.
16. Wait 3 seconds.
17. Press Escape.
18. Wait 3 seconds.
19. Press Escape.
20. Wait 3 seconds.
21. Press Escape.
22. Wait up to 300 seconds for the main menu.
23. Wait 3 seconds.
24. Take a screenshot called "main menu".
25. Check that the screenshot shows the main menu of Star Trek: Armada II (Single Player, Instant Action, Multiplayer, Options ...), not a black or garbled screen.
26. Click at 640,90.
27. Wait for "GAME SETUP".
28. Wait 2 seconds.
29. Click "LAUNCH".
30. Wait 60 seconds.
31. Take a screenshot called "match".
32. Check that the screenshot shows a space battle map of Star Trek: Armada II drawn by the game (stars, ships or stations, and the game's own interface), not a black or garbled screen and not a second copy of the interface drawn over the game's.
33. Expect "Render3D.log" not contain "frame: targets".
34. Expect "UI.log" contains "runs stock: no ".
35. Expect "Display8.log" not contain "PANIC".
36. Quit the game.
37. Expect the game to have exited.
38. Expect no crash.
