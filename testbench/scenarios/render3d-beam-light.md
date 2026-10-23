# render3d: phaser beams light the hulls and rocks they pass, paused frames

Resolutions: 1280x720
Mod: refit
Launch: -nointro a2_borg01
Clock: bench
Setup: render3d/bench-ini.sh beam-pass Seed=7 Step=16 Pause=200
Timeout: 12 min

The `beam-pass` scene (a Galaxy class firing its phasers at a Borg cube past a parked
destroyer on one side and an asteroid field on the other) paused on fixed ticks, for
render3d's phaser beams as line lights (`render3d/README.md`, "Clustered lights",
"Beams"): the destroyer's plating and the rocks along the line of fire take the
beam's light evenly along it, brighter only at the emitter and where the beam hits
the cube's shield. Line lights run with the clustered lights, on Vulkan and Direct3D
12 only: run it on Display8's stub with `display/stub-scenario.sh render3d-beam-light
stub Backend=vulkan` (with `A2TEST_VULKAN_GPU=1` on the GPU), and for the A/B with
`render3d/bench-ini.sh Lights:BeamLight=0` added before `beam-pass`. On Direct3D 9
only the two ends light.

1. Launch the game.
2. Wait up to 240 seconds for "Scene.log" to say "paused at tick 200".
3. Wait 8 seconds.
4. Take a screenshot called "t200".
5. Scene "pause-at 240".
6. Scene "resume".
7. Wait up to 60 seconds for "Scene.log" to say "paused at tick 240".
8. Wait 4 seconds.
9. Take a screenshot called "t240".
10. Expect no crash.
