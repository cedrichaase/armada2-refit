# render3d: a fight inside a nebula, the gas lit by the fire in it, timed

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: render3d/bench-ini.sh Render3D:Timing=1 Render3D:TimingEvery=4 nebula-battle Seed=7 Step=16 Pause=240
Timeout: 12 min

The `nebula-battle` scene (two Borg cubes and eight ships firing at them in the middle
of the Mutara nebula) for render3d's light in the gas (`render3d/README.md`,
"Nebulae", `[Nebulae] Lit=`): one paused frame, a run of frames in motion (a light
that moves must not trail its glow through the frames added up), and a ship blown up
inside the gas. With `Timing=1`, `Render3D.log` has the GPU time per step (`gas: march`
holds the lit march; `light clusters` the cull). Light in the gas runs on Vulkan and
Direct3D 12 only: run it on Display8's stub with
`display/stub-scenario.sh render3d-nebula-lit stub Backend=vulkan` (or `Backend=d3d12`),
and for the A/B with `render3d/bench-ini.sh Nebulae:Lit=0` added before `nebula-battle`.
On Direct3D 9 it is the gas as before.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Wait 14 seconds.
4. Take a screenshot called "paused".
5. Scene "resume".
6. Wait 2 seconds.
7. Take a screenshot called "motion 1".
8. Wait 1 second.
9. Take a screenshot called "motion 2".
10. Wait 1 second.
11. Take a screenshot called "motion 3".
12. Wait 1 second.
13. Take a screenshot called "motion 4".
14. Scene "kill sov".
15. Wait 1 second.
16. Take a screenshot called "breach 1".
17. Wait 1 second.
18. Take a screenshot called "breach 2".
19. Wait 2 seconds.
20. Take a screenshot called "breach 3".
21. Expect the game is still running.
22. Expect no crash.
