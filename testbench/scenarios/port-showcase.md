# Port check: the showcase battle with the HUD, one paused frame

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1
Timeout: 10 min

The frame a port of the renderer is judged by (`testbench/README.md`, "Baselines";
`gpu/README.md`): `showcase` paused on one tick with the bench clock, so two launches
draw the same picture (`bench-determinism`), with the Federation HUD of ours over it.
Record it from the tree before a port and compare the tree after:

    ./a2test run port-showcase --install <before>
    ./a2test baseline record port-showcase --from <that run's case>
    ./a2test run port-showcase --install <after>
    ./a2test baseline compare port-showcase --from <this run's case>

`port-nebula` and `port-firing` are the same for two more scenes.

Frozen references (phase 10 X4, `$A2_DATA/baselines`; assets, never in git), recorded on
round-e 4084eb4 at 1280x720 through `display/stub-scenario.sh <scenario> stub Backend=<b>`:
`p10-vk-port-showcase`, `-port-nebula`, `-port-firing` (the GPU's Vulkan, the gate after every
package: RMSE 0; `p10-vk-port-showcase` is RMSE 0 against `roundD-base-port-showcase`) and
`p10-d12-port-showcase`, `-port-nebula`, `-port-firing` (Direct3D 12 with every module on, the
engine draws nothing: RMSE 0 against itself; against Vulkan's 0.48 on the showcase).

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Expect "Render3D.log" contains "clock: bench".
4. Wait 5 seconds.
5. Take a screenshot called "showcase".
6. Dump the frame called "showcase".
7. Expect no crash.
