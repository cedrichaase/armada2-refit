# The stub falls back: Vulkan and Direct3D 12 fail at the first CreateDevice, Direct3D 9 draws

Resolutions: 1280x720
Reference: 1280x720
Mod: refit
Setup: display/bench-mode.sh stub Backend=auto FailBackend=vulkan,d3d12
Timeout: 20 min

Run with `--install .`: Display8 in stub mode with `Backend=auto`: Vulkan, then
Direct3D 12, then Direct3D 9 (`display/README.md`, "Choosing a backend").
`FailBackend=vulkan,d3d12`, honoured only in a bench clone, makes Vulkan and Direct3D
12 fail on purpose once each one's device, swap chain and frame are all made, so the
engine's first `CreateDevice` has to take each down whole and make the Direct3D 9
device on the same window, without the engine noticing. Direct3D 12 runs on the GPU
here (vkd3d-proton). The plugins must then draw on Direct3D 9: they are told
no backend until the device is made. Vulkan is tried on lavapipe unless
`A2TEST_VULKAN_GPU=1` (`display/bench-mode.sh`).

    ./a2test run display-fallback --install . -j 1

Other landings, with the backend-specific lines below changed to match, through
`display/stub-scenario.sh display-fallback stub FailBackend=vulkan` (Vulkan fails,
Direct3D 12 draws).

1. Launch the game with "".
2. Wait up to 120 seconds for "Display8.log" to say "stub: the backend is".
3. Expect "Display8.log" contains "Vulkan could not start: FailBackend=vulkan".
4. Expect "Display8.log" contains "vulkan: what was made is destroyed again".
5. Expect "Display8.log" contains "Direct3D 12 could not start: FailBackend=d3d12".
6. Expect "Display8.log" contains "d3d12: what was made is released again".
7. Expect "Display8.log" contains "stub: the backend is Direct3D 9, after vulkan, then d3d12 could not start".
8. Expect "Display8.log" not contain "no backend could start".
9. Wait up to 60 seconds for "UI.log" to say "gfx: ready on the d3d9 device".
10. Wait 8 seconds.
11. Take a screenshot called "intro".
12. Press Escape.
13. Wait 3 seconds.
14. Press Escape.
15. Wait 3 seconds.
16. Press Escape.
17. Wait 3 seconds.
18. Press Escape.
19. Wait up to 300 seconds for the main menu.
20. Wait 3 seconds.
21. Take a screenshot called "main menu".
22. Expect "UI.log" not contain "standing down: the shell runs stock".
23. Click the instant action cube.
24. Wait for "GAME SETUP".
25. Wait 2 seconds.
26. Click "LAUNCH".
27. Wait up to 180 seconds for "UI.log" to say "minimap: ours at".
28. Wait 15 seconds.
29. Take a screenshot called "match".
30. Expect "Render3D.log" contains "frame: targets 1280x720".
31. Expect "Render3D.log" contains "gpu: draws save and restore a recorded state block".
32. Expect "Render3D.log" not contain "gpu: on Vulkan".
33. Quit the game.
34. Expect the game to have exited.
35. Expect no crash.
