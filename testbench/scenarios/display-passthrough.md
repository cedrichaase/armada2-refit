# Passthrough: render3d's hulls module off on the stub, the engine's ships drawn by Display8

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: display/bench-mode.sh stub Passthrough=1 render3d/bench-ini.sh Modules:hulls=0 showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 15 min

Run with `--install .`. Display8 in stub mode with `Passthrough=1` (`display/README.md`,
"Passthrough"), render3d with its hulls module off (`[Modules] hulls=0`): every hull is
handed back to the engine, whose Direct3D 8 draws the stub used to drop, so the ships
vanished. Through the passthrough they are drawn into render3d's scene pass, as on the
forward chain. The showcase scene paused on one tick with the bench clock, as
`port-showcase`. On the bench the stub's Vulkan is the GPU with the validation layer (testbench
3.70.0; `--vulkan-device lavapipe` for Mesa's CPU Vulkan).

The same scene through the forward chain (`display-passthrough-forward`) is the
reference:

    ./a2test run display-passthrough-forward --install .
    ./a2test baseline record passthrough-hulls --from <that run's case>
    ./a2test run display-passthrough --install .
    ./a2test baseline compare passthrough-hulls --from <this run's case>

1. Launch the game.
2. Wait up to 600 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "module hulls: off".
4. Wait 5 seconds.
5. Take a screenshot called "showcase".
6. Expect "Display8.log" contains "passthrough: the engine's first draw, into a plugin's pass".
7. Expect "Display8.log" not contain "PASSTHROUGH an engine draw while a plugin records".
8. Expect "Display8.log" not contain "vulkan 0x".
9. Expect no crash.
