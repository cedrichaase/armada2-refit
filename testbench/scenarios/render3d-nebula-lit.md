# render3d: a fight inside a nebula, the gas lit by the fire in it, timed

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: render3d/bench-ini.sh Render3D:Timing=1 Render3D:TimingEvery=4 nebula-battle Seed=7 Step=16 Pause=240
Timeout: 12 min

The `nebula-battle` scene (two Borg cubes and eight ships firing at them in the middle
of the Mutara nebula) for render3d's light inside the gas (`render3d/README.md`,
"Nebulae", `[Nebulae] Lit=`): a paused frame (torpedoes, pulses and phaser beams in
the gas), a ship blown up in the gas and paused on a fixed tick after it (the two
frames repeat from run to run under the bench clock), then frames in motion (a light
that moves must not trail its glow through the frames added up). With `Timing=1`,
`Render3D.log` has the GPU time per step (`gas: march` holds the lit march;
`light clusters` the cull). Light in the gas runs on Vulkan and Direct3D 12 only: run
it on Display8's stub with `display/stub-scenario.sh render3d-nebula-lit stub
Backend=vulkan` (or `Backend=d3d12`), and for the A/B with `render3d/bench-ini.sh
Nebulae:Lit=0` added before `nebula-battle`. On Direct3D 9 it is the gas as before.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick 240".
3. Wait 14 seconds.
4. Take a screenshot called "paused".
5. Scene "kill sov".
6. Scene "pause-at 262".
7. Scene "resume".
8. Wait up to 60 seconds for "Scene.log" to say "paused at tick 262".
9. Wait 14 seconds.
10. Take a screenshot called "breach".
11. Scene "resume".
12. Wait 2 seconds.
13. Take a screenshot called "motion 1".
14. Wait 1 second.
15. Take a screenshot called "motion 2".
16. Wait 1 second.
17. Take a screenshot called "motion 3".
18. Wait 1 second.
19. Take a screenshot called "motion 4".
20. Expect the game is still running.
21. Expect no crash.
