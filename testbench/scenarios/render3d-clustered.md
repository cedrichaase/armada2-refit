# render3d: clustered lights in a big battle, one paused frame, timed

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: render3d/bench-ini.sh Render3D:Timing=1 Render3D:TimingEvery=4 battle Seed=7 Step=16 Pause=240
Timeout: 10 min

The `battle` scene (sixteen ships and a planet, a hundred and more lights at once)
paused on one tick, for render3d's clustered lights (`render3d/README.md`, "Clustered
lights"): the shot, and `Timing=1`'s report of the GPU time per step in
`Render3D.log` (`light clusters` is the cull; `objects` holds the hulls' shading).
Clustered lights run on Vulkan and Direct3D 12 only, so run it on Display8's stub with
`display/stub-scenario.sh render3d-clustered stub Backend=vulkan` (or `Backend=d3d12`),
and for the A/B with `render3d/bench-ini.sh Lights:Clustered=0` added before `battle`.
On Direct3D 9 it is the scene as before.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick".
3. Wait 14 seconds.
4. Take a screenshot called "battle".
5. Expect no crash.
