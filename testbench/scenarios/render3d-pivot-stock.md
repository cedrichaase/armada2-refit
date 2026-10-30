# The tactical camera's pivot marker given back to the engine, through the passthrough

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh stub Passthrough=1 render3d/bench-ini.sh Overlays:Pivot=0 fog
Timeout: 8 min

`render3d-pivot` with `[Overlays] Pivot=0`, on Display8's stub with `Passthrough=1`:
render3d leaves the pivot marker to `TacticalCamera::Render`, and the engine's draw of
it, which the stub would drop, is drawn by Display8 as Direct3D 8 would. Run it on a
checkout: `./a2test run render3d-pivot-stock --install .`. Judge the shot "pivot":
stock's small flat rose-red disc at the middle of the view, about 640,358 (compare
`$A2_DATA/notes/pivot-marker/stock-crop-marker-8x.png`); never render3d's ring.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "overlays: pivot marker 0".
5. Move the mouse to 640,200.
6. Press Caps_Lock.
7. Wait 3 seconds.
8. Press Right 10 times.
9. Press Down 12 times.
10. Press Left 25 times.
11. Wait 4 seconds.
12. Take a screenshot called "pivot".
13. Expect "Render3D.log" not contain "overlays: pivot marker drawn".
14. Expect "Display8.log" not contain "PASSTHROUGH an engine draw while a plugin records".
15. Expect the game is still running.
16. Quit the game.
17. Expect no crash.
