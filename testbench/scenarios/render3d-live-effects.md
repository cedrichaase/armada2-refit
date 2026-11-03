# render3d: the effects module switched off and on in a running game

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 12 min

`render3d/README.md`, "Modules and settings" and "effects", "Hooks". `Modules.effects` is
live: its hooks (the debris' `Fragment` calls, the repair welds' two calls and the shield
hits' tap) go in at the module's first switch-on, each behind the module's gate, and
what the module draws through the feature loop (explosions, shields, bees, the station
being built) is handed back by the registry. A flip off hands every effect to the engine
and drops the debris (the module's own simulation); the shield hits, welds and
construction edges are keyed to the engine's own records and stay, so the flip back on
picks the effects up where they are. Effects already in flight finish in the look they
started with, and the next spawned ones are the module's. The firing scene is not used
here: a craft that died while the module was on left no chunks of the engine's, so
its debris differs after a flip off.

Paused at one tick, with the bench clock, so the frames repeat. Off, live, must equal
off at load (`roundD-base-port-showcase-effects0`, recorded with
`Overlay: Render3D.Modules:effects=0` at launch) at RMSE 0. The frame after the flip back
is held to the base where the effects on screen are stateless reads of the engine's own
lists (explosions, shield hits), which they are in this scene.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Expect "Render3D.log" contains "effects: shields: the weapons' hits noted at:".
5. Wait 5 seconds.
6. Take a screenshot called "showcase".
7. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
8. Overlay Render3D.Modules:effects=0
9. Wait up to 20 seconds for the logs the next steps expect.
10. Expect "Render3D.log" contains "settings: applied Modules.effects=0".
11. Wait 3 seconds.
12. Take a screenshot called "showcase".
13. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-effects0" within rmse 0.
14. Overlay --unset Render3D.Modules:effects
15. Wait up to 20 seconds for the logs the next steps expect.
16. Expect "Render3D.log" contains "settings: applied Modules.effects=1".
17. Wait 3 seconds.
18. Take a screenshot called "showcase".
19. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase" within rmse 0.
20. Overlay Render3D.Modules:effects=0
21. Wait 3 seconds.
22. Take a screenshot called "showcase".
23. Expect the shot "showcase" matches the baseline "roundD-base-port-showcase-effects0" within rmse 0.
24. Expect the game is still running.
25. Expect no crash.
