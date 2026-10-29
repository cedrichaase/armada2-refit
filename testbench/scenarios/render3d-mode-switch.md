# render3d follows a display mode changed in the middle of a mission

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh forward testbench/scene/bench-setup.sh showcase
Timeout: 8 min

`render3d/README.md`. Run it on a checkout: `./a2test run render3d-mode-switch --install .`.
A display mode chosen in Graphics Settings makes the engine give up its device and
create a new one, so everything `Render3D.asi` keeps on the device (the scene target,
its depth, the bloom chain, the sky's cube, the nebulae's noise, the modules' buffers)
has to be dropped and made again at the new size. The log says it did ("a new device",
then the targets at 800x600, the one 4:3 mode the list offers at any size: at 720p
it has no 1024x768); the display8 stub offers one mode (the desktop's), so this runs on the
forward chain (`Setup:` below); ten seconds after the log lines the first frame at the new
size is drawn (the nebula noise is made again), so the shot waits for it. The shot after the switch must show the same scene as
before it, pillarboxed into the 1080p output, not black and not stretched.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "frame: targets".
5. Take a screenshot called "before".
6. Press Escape.
7. Wait for the options menu.
8. Click "Graphics Settings".
9. Wait for "Display Mode".
10. Click the display mode box.
11. Wait 2 seconds.
12. Click "800x600x32".
13. Wait 3 seconds.
14. Click "Previous Menu".
15. Wait for the options menu.
16. Press Escape.
17. Wait up to 10 seconds for the logs the next steps expect.
18. Expect "Render3D.log" contains "frame: a new device".
19. Expect "Render3D.log" contains "frame: targets 800x600".
20. Wait 10 seconds.
21. Take a screenshot called "after".
22. Check that the 3D view shows the same battle as the "before" screenshot (Borg cubes, a planet, Federation ships firing, a starry sky with nebula), drawn at 4:3 (between black bars on a wide screen), not black, not stretched and with no garbled or missing parts.
23. Wait 20 seconds.
24. Expect the game is still running.
25. Expect "Render3D.log" not contains "PANIC".
26. Quit the game.
27. Expect no crash.
