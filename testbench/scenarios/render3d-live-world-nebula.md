# render3d: the world switched live in the nebula scene, the gas and its lights back

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh nebula Seed=7 Step=16 Pause=200
Timeout: 14 min

`render3d/README.md`, "World" ("Hooks"), the nebulae's lights. As `render3d-live-world`
on the nebula scene: `Modules.world` flipped off through the overlay is the frame of the
same key at load (`roundD-base-port-nebula-world0`), where the engine's own
`Nebula::Simulate_Nebula_Lights` runs again through the hook's hand-back, then back on
is the base (`roundD-base-port-nebula`); twice. The sim is paused, so what the engine's own light
simulation would have done in the ticks that ran with the world on is not redone (bench, 2026-11-03: rmse
20 against the at-load frame, 1.2 back on, both steady in time): the bounds here are 25 and 2, not 0. `World.Nebulae` is flipped once and back
to the base.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Wait 5 seconds.
5. Take a screenshot called "nebula".
6. Expect the shot "nebula" matches the baseline "roundD-base-port-nebula" within rmse 0.
7. Overlay Render3D.Modules:world=0
8. Wait up to 20 seconds for the logs the next steps expect.
9. Expect "Render3D.log" contains "settings: applied Modules.world=0"
10. Wait 3 seconds.
11. Take a screenshot called "nebula".
12. Expect the shot "nebula" matches the baseline "roundD-base-port-nebula-world0" within rmse 25.
13. Overlay --unset Render3D.Modules:world
14. Wait 3 seconds.
15. Take a screenshot called "nebula".
16. Expect the shot "nebula" matches the baseline "roundD-base-port-nebula" within rmse 2.
17. Overlay Render3D.Modules:world=0
18. Wait up to 20 seconds for the logs the next steps expect.
19. Expect "Render3D.log" contains "settings: applied Modules.world=0"
20. Wait 3 seconds.
21. Take a screenshot called "nebula".
22. Expect the shot "nebula" matches the baseline "roundD-base-port-nebula-world0" within rmse 25.
23. Overlay --unset Render3D.Modules:world
24. Wait 3 seconds.
25. Take a screenshot called "nebula".
26. Expect the shot "nebula" matches the baseline "roundD-base-port-nebula" within rmse 2.
27. Overlay Render3D.World:Nebulae=0
28. Wait up to 20 seconds for the logs the next steps expect.
29. Expect "Render3D.log" contains "World.Nebulae=0"
30. Wait 3 seconds.
31. Overlay --unset Render3D.World:Nebulae
32. Wait 3 seconds.
33. Take a screenshot called "nebula".
34. Expect the shot "nebula" matches the baseline "roundD-base-port-nebula" within rmse 2.
35. Expect no crash.
