# render3d draws a loaded freighter: the latinum trail and the cargo markings

Resolutions: 16:9
Mod: refit
Launch: -nointro a2_borg01
Setup: testbench/scene/bench-setup.sh freighters
Timeout: 6 min

`render3d/README.md`, "particles" (Freighters' latinum spray) and "hulls" (A freighter's
cargo markings, alpha-tested meshes in old SODs); GitHub issue #2. Run it on a checkout:
`./a2test run render3d-freighter-trail --install . --install testbench/scene --res
1280x720`. The `freighters` scene parks the five freighters; `cargo` fills their holds.
A freighter carrying latinum trails a golden spray from one hardpoint, densest when the
hold is full; the Borg freighter has none. Every resource lights the cargo pods in its
colour, up to the fill (the Federation's pods one by one; the Klingon's all at once
when full, the Romulan's from 10 %): yellow for latinum, pale blue for dilithium. Before
render3d 0.16.2 there was no trail and the pods never lit.

1. Launch the game.
2. Wait 50 seconds.
3. Expect "Scene.log" contains "scene ready".
4. Expect "Render3D.log" contains "particles: ffreight: latinum spray latinum_spray at hp08".
5. Expect "Render3D.log" contains "particles: kfreight: latinum spray latinum_spray at hp10".
6. Expect "Render3D.log" not contains "particles: bfreight: latinum spray".
7. Expect scene "cargo kli latinum full" answers "ok: latinum 75.0".
8. Expect scene "cargo fed dilithium 75" answers "ok: dilithium 75.0".
9. Expect scene "cargo rom dilithium 20" answers "ok: dilithium 20.0".
10. Expect scene "cargo borg latinum full" answers "ok: latinum 75.0".
11. Wait 5 seconds.
12. Scene "pause; orbit kli 120 35 70".
13. Wait 2 seconds.
14. Take a screenshot called "klingon-full".
15. Check that a Klingon freighter (red plating) shows a long glowing yellow plume of star-shaped particles streaming from one end, and the five lattice-covered cargo pods along its side glow yellow.
16. Scene "orbit fed 120 35 70".
17. Wait 2 seconds.
18. Take a screenshot called "federation-dilithium-half".
19. Check that a grey Federation freighter shows no yellow plume and that some, not all, of the round cargo hatches along its hull glow pale blue while the others are dark.
20. Scene "orbit 2240 0 2000 180 25 420".
21. Wait 2 seconds.
22. Take a screenshot called "row".
23. Check that of the five freighters in a row only the second from the left (the red Klingon one) trails a yellow plume; the rightmost, a grey-green Borg box, has none.
24. Scene "resume; engines kli on; goto kli 2120 0 3400".
25. Wait 6 seconds.
26. Scene "pause; orbit kli 200 30 220".
27. Wait 2 seconds.
28. Take a screenshot called "klingon-moving".
29. Expect the game is still running.
30. Expect "Render3D.log" not contains "PANIC".
31. Quit the game.
32. Expect no crash.
