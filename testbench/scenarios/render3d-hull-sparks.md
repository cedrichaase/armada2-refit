# render3d: hull sparks in the Mutara gas, and the Borg's damaged sensors held steady

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Setup: display/bench-mode.sh stub Backend=vulkan nebula-battle Seed=7
Timeout: 10 min

The `nebula-battle` scene, run on Display8's Vulkan stub, where the clustered lights
light the gas (`render3d/README.md`, "Light inside the gas" and "Damaged sensors").
It covers two looks the user judged in game on 2026-10-23.

**The shields-down discharges and the welds as sparks.** The cubes' healing is turned
off, so the battle takes their shields down and their repairs weld. In the
"shields-down" shot, the gas about the cube shows at most a tight glow at each
discharge (`Gas::Spark`, `[Nebulae] LitSpark=`). Before the fix it showed a wide round
blue ball.

**The damaged sensors.** The attackers stop firing and a Borg scout is spawned in the
gas, where the Mutara takes its sensors down. Its two green `sensor` sprite nodes are
the subject of the "sensors" shots, two frames 0.1 s apart. In both frames they should
be the same still star, at half stock's size, and the plating near them should take a
small green light (`[Hulls] SensorSteady=`, `SensorSize=`, `SensorLight=`,
`SensorLightReach=`). Before the fix the spark flipbook flickered.

Run it with `A2TEST_VULKAN_GPU=1 ./a2test run render3d-hull-sparks --install .`, on
the GPU: without the flag the game runs on lavapipe.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "scene ready".
3. Scene "heal cube1 off".
4. Scene "heal cube2 off".
5. Wait up to 120 seconds for "Render3D.log" to say "shields down on bbattle1".
6. Wait 6 seconds.
7. Scene "orbit cube1 200 18 400".
8. Wait 3 seconds.
9. Take a screenshot called "shields-down".
10. Scene "weapons sov off".
11. Scene "weapons galaxy off".
12. Scene "weapons martok off".
13. Scene "weapons negh off".
14. Scene "weapons gal2 off".
15. Scene "weapons ente2 off".
16. Scene "weapons card off".
17. Scene "weapons rom off".
18. Scene "spawn sc bscout 2200 0 2300 0 2".
19. Wait up to 30 seconds for "Render3D.log" to say "damaged-sensor light(s) in the view".
20. Scene "orbit sc 60 40 90".
21. Wait 3 seconds.
22. Take a screenshot called "sensors 1".
23. Wait 0.1 seconds.
24. Take a screenshot called "sensors 2".
25. Expect the game is still running.
26. Expect no crash.
