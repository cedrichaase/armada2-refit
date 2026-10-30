# Passthrough: the engine's map puffs move as on the forward chain

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: display/bench-mode.sh stub Passthrough=1 render3d/bench-ini.sh Modules:world=0 Modules:particles=0 Modules:hulls=0 Modules:effects=0 Modules:ordnance=0 Modules:specials=0 Modules:sky=0 Modules:overlays=0 nebula-cerulean Seed=7 Step=16 Pause=200
Timeout: 15 min

Run with `--install .`, after `display-passthrough-nebula-forward`, whose baseline
`passthrough-nebula` it is compared against (`display/README.md`, "Passthrough",
"Map nebulae"). Display8 in stub mode with `Passthrough=1` and every render3d module
off: the engine draws the cerulean nebula's puffs (`mnebula10`, `Mnebula5`) and
Display8 draws them on Vulkan. Each frame, at ticks 200, 202 and 220, matches the
forward chain's at the same tick, so the puffs stand and move exactly as there; the
paused frame stands still. Measured 2026-10-30: RMSE 0.93/255 against the forward
frame (additive blending's last bit over the gas, the ship's edges), the frame-to-frame
change over 2 ticks 1.24 forward, 1.24 here.

1. Launch the game.
2. Wait up to 600 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "module particles: off".
4. Wait 5 seconds.
5. Take a screenshot called "t200".
6. Wait 2 seconds.
7. Take a screenshot called "t200b".
8. Expect shots "t200" and "t200b" to match within 0.
9. Scene "pause-at 202; resume".
10. Wait up to 60 seconds for "Scene.log" to say "paused at tick 202".
11. Wait 3 seconds.
12. Take a screenshot called "t202".
13. Scene "pause-at 220; resume".
14. Wait up to 60 seconds for "Scene.log" to say "paused at tick 220".
15. Wait 3 seconds.
16. Take a screenshot called "t220".
17. Expect the screenshot "t200" matches baseline "passthrough-nebula" within RMSE 1.5 with at most 1% of pixels over 4.
18. Expect the screenshot "t202" matches baseline "passthrough-nebula" within RMSE 1.5 with at most 1% of pixels over 4.
19. Expect the screenshot "t220" matches baseline "passthrough-nebula" within RMSE 1.5 with at most 1% of pixels over 4.
20. Expect "Display8.log" contains "passthrough: the engine's first draw".
21. Expect "Display8.log" not contain "PASSTHROUGH a texture transform".
22. Expect "Display8.log" not contain "PASSTHROUGH generated texture coordinates".
23. Expect "Display8.log" not contain "vulkan 0x".
24. Expect no crash.
