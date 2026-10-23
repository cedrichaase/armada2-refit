# The stub falls back: Vulkan fails at the first CreateDevice, Direct3D 9 draws

Resolutions: 1280x720
Reference: 1280x720
Mod: refit
Setup: display/bench-mode.sh stub Backend=auto FailBackend=vulkan
Timeout: 20 min

Run with `--install .`: Display8 in stub mode with `Backend=auto`, which on the bench
(Wine on Linux) is Vulkan, then Direct3D 9 (`display/README.md`, "Choosing a backend").
`FailBackend=vulkan`, honoured only in a bench clone, makes Vulkan fail on purpose once
its device, swap chain and frame are all made, so the engine's first `CreateDevice`
has to take Vulkan down whole and make the Direct3D 9 device on the same window,
without the engine noticing. The plugins must then draw on Direct3D 9: they are told
no backend until the device is made. Vulkan is tried on lavapipe unless
`A2TEST_VULKAN_GPU=1` (`display/bench-mode.sh`).

    ./a2test run display-fallback --install . -j 1

Other orders, with the backend-specific lines below changed to match, through
`display/stub-scenario.sh display-fallback stub Backend=d3d12,vulkan
FailBackend=d3d12` (Direct3D 12 fails, Vulkan draws).

1. Launch the game with "".
2. Wait up to 120 seconds for "Display8.log" to say "stub: the backend is".
3. Expect "Display8.log" contains "Vulkan could not start: FailBackend=vulkan".
4. Expect "Display8.log" contains "vulkan: what was made is destroyed again".
5. Expect "Display8.log" contains "stub: the backend is Direct3D 9, after vulkan could not start".
6. Expect "Display8.log" not contain "no backend could start".
7. Wait up to 60 seconds for "UI.log" to say "gfx: ready on the d3d9 device".
8. Wait 8 seconds.
9. Take a screenshot called "intro".
10. Press Escape.
11. Wait 3 seconds.
12. Press Escape.
13. Wait 3 seconds.
14. Press Escape.
15. Wait 3 seconds.
16. Press Escape.
17. Wait up to 300 seconds for the main menu.
18. Wait 3 seconds.
19. Take a screenshot called "main menu".
20. Expect "UI.log" not contain "standing down: the shell runs stock".
21. Click the instant action cube.
22. Wait for "GAME SETUP".
23. Wait 2 seconds.
24. Click "LAUNCH".
25. Wait up to 180 seconds for "UI.log" to say "minimap: ours at".
26. Wait 15 seconds.
27. Take a screenshot called "match".
28. Expect "Render3D.log" contains "frame: targets 1280x720".
29. Expect "Render3D.log" contains "gpu: draws save and restore a recorded state block".
30. Expect "Render3D.log" not contain "gpu: on Vulkan".
31. Quit the game.
32. Expect the game to have exited.
33. Expect no crash.
