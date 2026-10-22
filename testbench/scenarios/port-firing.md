# Port check: the firing scene, one paused frame

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: testbench/scene/bench-setup.sh firing Seed=7 Step=16 Pause=200
Timeout: 10 min

As `port-showcase`, for beams, torpedoes, shield hits and their sparks.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Wait 5 seconds.
4. Take a screenshot called "firing".
5. Dump the frame called "firing".
6. Expect no crash.
