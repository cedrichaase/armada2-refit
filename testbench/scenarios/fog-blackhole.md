# A black hole under the fog of war blends with the fog about it

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh fog-blackhole
Timeout: 6 min

Issue #3; `render3d/README.md`, "overlays" (ghosts). Run it on a checkout:
`./a2test run fog-blackhole --install .`. The scene
(`testbench/scene/scenes/fog-blackhole.ini`) parks the player's scout by a black hole
(`mblkhole`) and a quantum singularity with the map's fog on, then sends it away, so
both are left as fog-of-war ghosts. Their meshes are `additive`: stock adds them at
half strength over the fog. Drawn opaque and lit like a hull, the black hole's ghost
was a near-black disc in the fog. Judge "hole-fogged" against "hole-seen": under the
fog the swirl is still there, a little dimmer, and the fog about it and inside the
disc's outline is the same grey (measured 2026-10-21 at the disc's rim: before the
fix 2 and 19 against 49 and 54 outside, after 53 and 61; stock 53 and 56).

1. Launch the game.
2. Wait up to 50 seconds for the logs the next steps expect.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "module overlays: on".
5. Take a screenshot called "hole-seen".
6. Scene "goto scout 6500 0 6500".
7. Wait up to 35 seconds for the logs the next steps expect.
8. Expect "Render3D.log" contains "overlays: ghost of mblkhole at 3000,-70,3000, 2 additive mesh(es)".
9. Expect "Render3D.log" contains "overlays: ghost of quantumsingularity".
10. Take a screenshot called "hole-fogged".
11. Expect the game is still running.
12. Quit the game.
13. Expect no crash.
