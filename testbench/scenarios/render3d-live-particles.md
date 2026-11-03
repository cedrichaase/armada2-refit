# render3d: the particles module switched off and on in a running game

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 12 min

`render3d/README.md`, "Modules and settings" and "particles", "Hooks". `Modules.particles` is
live: its hooks (the engine's `RenderParticle` slot and the billboard effects' call) go
in at the module's first switch-on, behind its gate, so a flip off hands every particle
back to the engine and the flip back on takes them again. Paused at one tick, with the
bench clock, so the frames repeat.

Off, live, must equal off at load (the baseline `roundD-base-port-showcase-particles0`,
recorded with `Overlay: Render3D.Modules:particles=0` at launch) at RMSE 0. Back on, the
module's simulation starts clean (what the engine and the module had in flight is not
carried across the switch), so the frame after the flip back is a check of the log and
the frame, not of the base: effects already in flight finish in the look they started
with, and the next spawned ones are the module's.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Expect "Render3D.log" contains "at ST3D_DeviceDirectX8::RenderParticle (0x6bc6ac) installed".
5. Wait 5 seconds.
6. Take a screenshot called "showcase".
7. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
8. Overlay Render3D.Modules:particles=0
9. Wait up to 20 seconds for the logs the next steps expect.
10. Expect "Render3D.log" contains "settings: applied Modules.particles=0".
11. Wait 3 seconds.
12. Take a screenshot called "showcase".
13. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-particles0" within rmse 0.
14. Overlay --unset Render3D.Modules:particles
15. Wait up to 20 seconds for the logs the next steps expect.
16. Expect "Render3D.log" contains "settings: applied Modules.particles=1".
17. Wait 3 seconds.
18. Take a screenshot called "showcase back".
19. Overlay Render3D.Modules:particles=0
20. Wait 3 seconds.
21. Take a screenshot called "showcase".
22. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-particles0" within rmse 0.
23. Expect the game is still running.
24. Expect no crash.
