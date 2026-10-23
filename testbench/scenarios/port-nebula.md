# Port check: the nebula scene, one paused frame

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh nebula Seed=7 Step=16 Pause=200
Timeout: 10 min

As `port-showcase`, for the volume march, the gas and their lights.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Wait 5 seconds.
4. Take a screenshot called "nebula".
5. Dump the frame called "nebula".
6. Expect no crash.
