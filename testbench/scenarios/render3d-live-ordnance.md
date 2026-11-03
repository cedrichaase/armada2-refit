# render3d: the ordnance module switched off and on in a running game

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh firing Seed=7 Step=16 Pause=200
Timeout: 12 min

`render3d/README.md`, "Modules and settings" and "Ordnance", "Hooks". `Modules.ordnance`
is live: the warp flash's hook goes in at the module's first switch-on and the
per-class `Entity::Render` slots when a class is first met, all behind its gate. A flip
off hands every shot to the engine (`Ordnance::RenderAll`, the warp sprite) and the
flip back on takes them again. The firing scene's bolt-hit sparks vary from one launch
to the next, so these frames are not held to a pixel baseline: the checks are the log,
that the frames are taken, and that the game goes on (`roundD-base-port-firing` is
compared by hand).

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Expect "Render3D.log" contains "at WarpSpriteEffect::Render (0x59cc6b) installed".
5. Wait 5 seconds.
6. Take a screenshot called "firing".
7. Overlay Render3D.Modules:ordnance=0
8. Wait up to 20 seconds for the logs the next steps expect.
9. Expect "Render3D.log" contains "settings: applied Modules.ordnance=0".
10. Wait 3 seconds.
11. Take a screenshot called "firing off".
12. Overlay --unset Render3D.Modules:ordnance
13. Wait up to 20 seconds for the logs the next steps expect.
14. Expect "Render3D.log" contains "settings: applied Modules.ordnance=1".
15. Wait 3 seconds.
16. Take a screenshot called "firing back".
17. Overlay Render3D.Modules:ordnance=0
18. Wait 3 seconds.
19. Take a screenshot called "firing off again".
20. Expect the game is still running.
21. Expect no crash.
