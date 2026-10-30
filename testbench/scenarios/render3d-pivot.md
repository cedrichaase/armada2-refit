# The tactical camera's pivot marker, in render3d's look

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh forward fog
Timeout: 8 min

`render3d/README.md`, "overlays" (the pivot marker). Run it on a checkout, with
Display8 in forward mode counting the engine's draws:
`./a2test run render3d-pivot --install .`. In the tactical view (Caps Lock), once the
camera is panned off the units, it turns about a point of the map and stock marks that
point with a small rose-red disc (`TacticalCamera::Render`, the instance at `+0xe0`).
With `[Overlays] Pivot=1` (shipped) render3d draws its own marker there, a thin glowing
ring with four ticks on the map's plane, and the engine draws nothing in the tactical
view. Judge the shot "pivot": the ring at the middle of the view, about 640,360; never
the stock disc. `render3d-pivot-stock` is the same view with `Pivot=0` on the stub, the
engine's disc through Display8's passthrough.

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "overlays: pivot marker 1".
5. Move the mouse to 640,200.
6. Press Caps_Lock.
7. Wait 3 seconds.
8. Press Right 10 times.
9. Press Down 12 times.
10. Press Left 25 times.
11. Wait up to 20 seconds for "Render3D.log" to say "overlays: pivot marker drawn".
12. Expect "Render3D.log" contains "overlays: pivot marker: ours".
13. Mark the display context "tactical".
14. Wait 4 seconds.
15. Take a screenshot called "pivot".
16. Expect the engine to draw nothing in "tactical".
17. Expect the game is still running.
18. Quit the game.
19. Expect no crash.
