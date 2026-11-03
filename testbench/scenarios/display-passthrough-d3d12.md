# Passthrough on Direct3D 12: render3d's hulls module off, the engine's ships drawn by Display8

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: display/bench-mode.sh stub Backend=d3d12 Passthrough=1 render3d/bench-ini.sh Modules:hulls=0 showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 15 min

Run with `--install .`. `display-passthrough` on Direct3D 12 (`Backend=d3d12`): Display8's
stub on its own Direct3D 12 device, `Passthrough=1`, render3d with its hulls module off
(`[Modules] hulls=0`), so every hull is handed back to the engine and Display8 records
the engine's draws (`dx12/pass.rs`) into the pass render3d published
(`display8v1_dx12_pass`, contract 1.4), as `vk/pass.rs` does on Vulkan. The showcase scene
paused on one tick with the bench clock, as `port-showcase`.

**This is the first thing to run after a change to `dx12/pass.rs`**: it is one paused
frame, and a Direct3D 12 game on the bench is on the GPU with no validation that reaches
the log (`testbench/README.md`). A `device removed` in `Display8.log` stops the work;
nothing else is run until it is fixed.

The references are the same scene on Vulkan (`display-passthrough`, recorded as the frozen
baseline `p10-vk-display-passthrough`) and the forward chain (the frozen
`p10-fwd-display-passthrough`, recorded by `display-passthrough-forward`):

    ./a2test baseline compare p10-vk-display-passthrough --from <this run's case>

The tolerance is RMSE 1.0 on the whole frame against Vulkan's (the two APIs filter and
round a little differently); against the forward chain, Vulkan's own figure
(1.64 on this scene with `PixelCentres=d3d9`) plus 0.5.

1. Launch the game.
2. Wait up to 600 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "module hulls: off".
4. Wait 5 seconds.
5. Take a screenshot called "showcase".
6. Expect "Display8.log" contains "passthrough: on Direct3D 12".
7. Expect "Display8.log" contains "passthrough: the engine's first draw, into a plugin's pass".
8. Expect "Display8.log" not contain "PASSTHROUGH an engine draw while a plugin records".
9. Expect "Display8.log" not contain "passed through on Vulkan and Direct3D 12 only".
10. Expect "Display8.log" not contain "device is gone".
11. Expect "Display8.log" not contain "pipeline could not be made".
12. Expect the screenshot "showcase" matches baseline "p10-vk-display-passthrough" within RMSE 1.0.
13. Expect no crash.
