# render3d: a breach inside the Mutara gas and one just outside it

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: display/bench-mode.sh stub Backend=vulkan nebula-battle Seed=7 Pause=240
Timeout: 14 min

How far a fireball lights the nebula's gas (`render3d/README.md`, "Light inside the
gas", `[Nebulae] LitExplosion=`, `LitExplosionCore=`, `LitExplosionFar=`), in the
`nebula-battle` scene on Display8's Vulkan stub, where the clustered lights light the
gas. Run it on the GPU with `A2TEST_VULKAN_GPU=1 ./a2test run
render3d-nebula-explosion --install .`; without the flag it runs on lavapipe.

**Inside.** The Sovereign (`sov`) blows up in the middle of the gas, paused 22 ticks
after (the `render3d-nebula-lit` scenario's "breach"), and again 60 ticks after. The
gas about the breach glows strongly, out to a few of the ship's radii; the rest of the
cloud in view takes a dim wash. Until render3d 0.27.0 the whole cloud in view went
white as far as the frame's edge (the user, in game, 2026-10-23: "out of proportion").

**Outside.** A second Sovereign (`out`), spawned some 100 units off the gas's west
edge, blows up in the same way, seen side on with the gas beside and behind it: the
gas's near edge is lit hot about the breach and fades across the cloud. This is the
look the user liked in game and it should stay.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick 240".
3. Scene "spawn out fbattle 1520 0 2350 0 1".
4. Scene "engines out off".
5. Scene "weapons out off".
6. Wait 14 seconds.
7. Take a screenshot called "inside before".
8. Scene "kill sov".
9. Scene "pause-at 262".
10. Scene "resume".
11. Wait up to 60 seconds for "Scene.log" to say "paused at tick 262".
12. Wait 14 seconds.
13. Take a screenshot called "inside".
14. Scene "pause-at 300".
15. Scene "resume".
16. Wait up to 60 seconds for "Scene.log" to say "paused at tick 300".
17. Wait 14 seconds.
18. Take a screenshot called "inside later".
19. Scene "pause-at 700".
20. Scene "resume".
21. Wait up to 120 seconds for "Scene.log" to say "paused at tick 700".
22. Scene "orbit 1520 0 2350 0 18 650".
23. Wait 14 seconds.
24. Take a screenshot called "outside before".
25. Scene "kill out".
26. Scene "pause-at 725".
27. Scene "resume".
28. Wait up to 60 seconds for "Scene.log" to say "paused at tick 725".
29. Wait 14 seconds.
30. Take a screenshot called "outside".
31. Scene "pause-at 763".
32. Scene "resume".
33. Wait up to 60 seconds for "Scene.log" to say "paused at tick 763".
34. Wait 14 seconds.
35. Take a screenshot called "outside later".
36. Expect the game is still running.
37. Expect no crash.
