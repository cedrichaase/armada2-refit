# Passthrough's reference: render3d's hulls module off, on the forward chain

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: display/bench-mode.sh forward render3d/bench-ini.sh Modules:hulls=0 showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 15 min

Run with `--install .`. `display-passthrough`'s scene with Display8 in forward mode: the
engine's draws of the hulls render3d hands back go to the real chain (d3d8to9, DXVK on
the GPU) and land in render3d's scene there, as they always have. What
`display-passthrough` is compared against (its text says how).

1. Launch the game.
2. Wait up to 600 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "module hulls: off".
4. Wait 5 seconds.
5. Take a screenshot called "showcase".
6. Expect no crash.
